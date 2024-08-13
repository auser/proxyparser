#![allow(unused)]
use std::{cell::RefCell, collections::HashSet};

use promkit::json::{JsonPathSegment, JsonStream};
use promkit::preset::query_selector::QuerySelector;
use promkit::serde_json;

use promkit::listbox::Listbox;
use promkit::PaneFactory;
use promkit::{
    crossterm::{
        event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
        style::{Attribute, Attributes, Color},
    },
    listbox,
    pane::Pane,
    snapshot::Snapshot,
    style::StyleBuilder,
    suggest::Suggest,
    switch::ActiveKeySwitcher,
    text, text_editor, Prompt, PromptSignal, Renderer,
};
use radix_trie::{Trie, TrieCommon};

use super::configs::ProxyConfig;

pub fn exec(input_stream: ProxyConfig) -> anyhow::Result<()> {
    let interact = Interact::try_new(input_stream)?;
    let mut prompt = interact.get_prompt();
    prompt.run()?;
    Ok(())
}

pub struct Interact {
    input_stream: Vec<serde_json::Value>,

    keymap: RefCell<ActiveKeySwitcher<Keymap>>,

    filter_editor: Snapshot<text_editor::State>,
    hint_message: Snapshot<text::State>,
    suggestions: listbox::State,
    suggest: Suggest,

    trie: FilterTrie,
}

impl Interact {
    pub fn try_new(input: ProxyConfig) -> anyhow::Result<Self> {
        let mut input_stream: Vec<serde_json::Value> = input.to_json();

        let mut trie = FilterTrie::default();
        trie.insert(".", input_stream.clone());

        let filter_editor = text_editor::State {
            texteditor: Default::default(),
            history: Default::default(),
            prefix: String::from("> "),
            mask: None,
            prefix_style: StyleBuilder::new().fgc(Color::Blue).build(),
            active_char_style: StyleBuilder::new().bgc(Color::Magenta).build(),
            inactive_char_style: StyleBuilder::new().build(),
            edit_mode: text_editor::Mode::Insert,
            word_break_chars: HashSet::from(['.', '|', '(', ')', '[', ']']),
            lines: Default::default(),
        };

        let hint_message = text::State {
            text: Default::default(),
            style: StyleBuilder::new()
                .fgc(Color::Green)
                .attrs(Attributes::from(Attribute::Bold))
                .build(),
        };

        let suggestions = listbox::State {
            listbox: listbox::Listbox::from_iter(Vec::<String>::new()),
            cursor: String::from("> "),
            active_item_style: Some(
                StyleBuilder::new()
                    .fgc(Color::Grey)
                    .bgc(Color::Yellow)
                    .build(),
            ),
            inactive_item_style: Some(StyleBuilder::new().fgc(Color::Grey).build()),
            lines: Some(1),
        };

        let keymap = RefCell::new(
            ActiveKeySwitcher::new("default", self::default as Keymap)
                .register("on_suggest", self::default),
        );

        let suggestable_keys = input_stream
            .clone()
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<String>>();
        println!("input_stream: {:?}", suggestable_keys);
        let suggest = Suggest::from_iter(suggestable_keys);

        Ok(Self {
            input_stream,
            keymap,
            filter_editor: Snapshot::new(filter_editor),
            hint_message: Snapshot::new(hint_message),
            suggestions,
            suggest,
            trie,
        })
    }

    fn parse_query_selector(text: &str, items: &Vec<String>) -> Vec<String> {
        text.parse::<usize>()
            .map(|query| {
                items
                    .iter()
                    .filter(|num| num.starts_with(text))
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .unwrap_or(items.clone())
    }

    pub fn get_prompt(self) -> Prompt<Self> {
        Prompt { renderer: self }
    }
}

pub type Keymap = fn(&Event, &mut Interact) -> anyhow::Result<PromptSignal>;

pub fn default(event: &Event, interact: &mut Interact) -> anyhow::Result<PromptSignal> {
    let filter_editor = interact.filter_editor.after_mut();

    match event {
        Event::Key(KeyEvent {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }) => {
            let query = filter_editor.texteditor.text_without_cursor().to_string();
            if let Some(mut candidates) = interact.suggest.prefix_search(query) {
                candidates.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));

                interact.suggestions.listbox = Listbox::from_iter(candidates);
                filter_editor
                    .texteditor
                    .replace(&interact.suggestions.listbox.get().to_string());

                interact.keymap.borrow_mut().switch("on_suggest");
            }
            // if let Some(mut candidates) = interact.suggest.prefix_search(query) {
            //     if candidates.len() == 1 {
            //         filter_editor.texteditor.insert_text(candidates[0]);
            //     }
            // }
        }
        Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }) => {
            return Ok(PromptSignal::Quit);
        }
        // Move
        Event::Key(KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }) => {
            filter_editor.texteditor.backward();
        }
        Event::Key(KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }) => {
            filter_editor.texteditor.forward();
        }
        // Erase
        Event::Key(KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }) => {
            filter_editor.texteditor.erase();
        }
        Event::Key(KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }) => {
            filter_editor.texteditor.erase_all();
        }

        // Insert
        Event::Key(KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
        | Event::Key(KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }) => match filter_editor.edit_mode {
            text_editor::Mode::Insert => {
                filter_editor.texteditor.insert(*ch);
            }
            text_editor::Mode::Overwrite => {
                filter_editor.texteditor.overwrite(*ch);
            }
        },
        _ => {
            interact.suggestions.listbox = Listbox::from_iter(Vec::<String>::new());
            interact.keymap.borrow_mut().switch("default");

            if let Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }) = event
            {
            } else {
                return default(event, interact);
            }
        }
    }

    Ok(PromptSignal::Continue)
}

impl promkit::Finalizer for Interact {
    type Return = String;

    fn finalize(&self) -> anyhow::Result<Self::Return> {
        Ok(self
            .filter_editor
            .after()
            .texteditor
            .text_without_cursor()
            .to_string())
    }
}

impl Renderer for Interact {
    fn evaluate(&mut self, event: &Event) -> anyhow::Result<PromptSignal> {
        let keymap = *self.keymap.borrow_mut().get();
        let signal = keymap(event, self)?;

        let filter = self
            .filter_editor
            .after()
            .texteditor
            .text_without_cursor()
            .to_string();

        // if filter != self.filter_editor.borrow_before().texteditor.text_without_cursor().to_string() {
        //     self.hint_message.reset_after_to_init();
        //     match self.trie.exact_search(&filter) {
        //         Some(nodes) => {
        //             self.update_hint_message(nodes);
        //         }
        //     }
        // }

        Ok(signal)
    }

    fn create_panes(&self, width: u16, height: u16) -> Vec<Pane> {
        vec![
            self.filter_editor.create_pane(width, height),
            self.hint_message.create_pane(width, height),
            self.suggestions.create_pane(width, height),
        ]
    }
}

#[derive(Default, Clone)]
pub struct FilterTrie(Trie<String, Vec<serde_json::Value>>);

impl FilterTrie {
    pub fn insert(&mut self, query: &str, json_nodes: Vec<serde_json::Value>) {
        self.0.insert(query.to_string(), json_nodes);
    }

    pub fn exact_search(&self, query: &str) -> Option<&Vec<serde_json::Value>> {
        self.0.get(query)
    }

    pub fn prefix_search(&self, query: &str) -> Option<&Vec<serde_json::Value>> {
        self.0
            .get_ancestor(query)
            .and_then(|subtrie| subtrie.value())
    }
}

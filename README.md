# ProxyParser

ProxyParser is a command-line tool designed to parse Nginx and Apache configuration files. It provides functionality to read, process, and convert configuration directives into a structured format.

## Features

- **Verbose Logging**: Control the level of logging output with the `--verbose` flag. Levels include:
  - `0`: Warn
  - `1`: Debug
  - `2`: Trace
- **File Processing**: Specify the directory containing configuration files and the file extension to look for.
- **Command Printing**: Optionally print the parsed configuration commands to the console.

## Usage

To use ProxyParser, run the following command in your terminal:

```bash
proxyparser --dir /path/to/config/dir --ext nginx --verbose 1 --print
```

This command will parse all Apache configuration files in the specified directory and print the parsed configuration commands to the console.

Optionally, you can use the `--print` flag to print the parsed `etcd` commands to the console.

Run `proxyparser --help` to see the full list of available options.
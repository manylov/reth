# `reth import`

```bash
$ reth import --help

Usage: reth import [OPTIONS]

Options:
      --config <FILE>
          The path to the configuration file to use.

      --datadir <DATA_DIR>
          The path to the data dir for all reth files and subdirectories.

          Defaults to the OS-specific data directory:

          - Linux: `$XDG_DATA_HOME/reth/` or `$HOME/.local/share/reth/`
          - Windows: `{FOLDERID_RoamingAppData}/reth/`
          - macOS: `$HOME/Library/Application Support/reth/`

      --chain <CHAIN_OR_PATH>
          The chain this node is running.

          Possible values are either a built-in chain or the path to a chain specification file.

          Built-in chains:
          - mainnet
          - goerli
          - sepolia

          [default: mainnet]

      --path <IMPORT_PATH>
          The path to a block file for import.

          The online stages (headers and bodies) are replaced by a file import, after which the
          remaining stages are executed.

Logging:
      --log.persistent
          The flag to enable persistent logs

      --log.directory <PATH>
          The path to put log files in

          [default: /Users/georgios/Library/Caches/reth/logs]

      --log.journald
          Log events to journald

      --log.filter <FILTER>
          The filter to use for logs written to the log file

          [default: debug]

Display:
  -v, --verbosity...
          Set the minimum log level.

          -v      Errors
          -vv     Warnings
          -vvv    Info
          -vvvv   Debug
          -vvvvv  Traces (warning: very verbose!)

  -q, --quiet
          Silence all log output
```

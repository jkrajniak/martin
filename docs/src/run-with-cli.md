## Command-line Interface

You can configure Martin using command-line interface. See `martin --help` or `cargo run -- --help` for more information.

```shell
Usage: martin [OPTIONS] [CONNECTION]...

Arguments:
  [CONNECTION]...
          Connection strings, e.g. postgres://... or /path/to/files

Options:
  -c, --config <CONFIG>
          Path to config file. If set, no tile source-related parameters are allowed

      --save-config <SAVE_CONFIG>
          Save resulting config to a file or use "-" to print to stdout. By default, only print if sources are auto-detected

  -s, --sprite <SPRITE>
          Export a directory with SVG files as a sprite source. Can be specified multiple times

  -f, --font <FONT>
          Export a font file or a directory with font files as a font source (recursive). Can be specified multiple times

  -k, --keep-alive <KEEP_ALIVE>
          Connection keep alive timeout. [DEFAULT: 75]

  -l, --listen-addresses <LISTEN_ADDRESSES>
          The socket address to bind. [DEFAULT: 0.0.0.0:3000]

  -W, --workers <WORKERS>
          Number of web server workers

  -b, --auto-bounds <AUTO_BOUNDS>
          Specify how bounds should be computed for the spatial PG tables. [DEFAULT: quick]

          Possible values:
          - quick: Compute table geometry bounds, but abort if it takes longer than 5 seconds
          - calc:  Compute table geometry bounds. The startup time may be significant. Make sure all GEO columns have indexes
          - skip:  Skip bounds calculation. The bounds will be set to the whole world

      --ca-root-file <CA_ROOT_FILE>
          Loads trusted root certificates from a file. The file should contain a sequence of PEM-formatted CA certificates

  -d, --default-srid <DEFAULT_SRID>
          If a spatial PG table has SRID 0, then this default SRID will be used as a fallback

  -p, --pool-size <POOL_SIZE>
          Maximum Postgres connections pool size [DEFAULT: 20]

  -m, --max-feature-count <MAX_FEATURE_COUNT>
          Limit the number of features in a tile from a PG table source

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

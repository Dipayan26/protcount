# protcount

Count the amino acid composition of a protein sequence.

Reads from an argument, a file (plain text or FASTA), or stdin, and reports the
count and percentage of each of the 20 standard amino acids.

## Install

```sh
curl -LsSf https://github.com/Dipayan26/protcount/releases/latest/download/install.sh | sh
```

Installs a prebuilt binary to `~/.local/bin`. No Rust toolchain needed.
Set `PROTCOUNT_INSTALL_DIR` to install somewhere else.

If you have Rust:

```sh
cargo install protcount
```

Or grab a binary directly from the [releases page](https://github.com/Dipayan26/protcount/releases).
Linux builds are statically linked with musl, so they run on older
distributions and HPC nodes without a matching glibc.

## Usage

```sh
protcount MKTAYIAKQRQISFVKSHFSRQLEERLGLIEVQ --sort
protcount --file hemoglobin.fasta --sort --nonzero
cat sequences.fasta | protcount --json
```

```
Standard residues: 33

   3AA  Name            Count     Pct
--------------------------------------------------
Q  Gln  Glutamine           4  12.12%  ######
E  Glu  Glutamic acid       3   9.09%  #####
I  Ile  Isoleucine          3   9.09%  #####
K  Lys  Lysine              3   9.09%  #####
...
```

### Options

| Flag | Description |
|---|---|
| `-f`, `--file <PATH>` | Read the sequence from a file instead of an argument |
| `-s`, `--sort` | Sort by count, highest first, instead of alphabetically |
| `-z`, `--nonzero` | Hide amino acids that do not occur |
| `-j`, `--json` | Emit JSON instead of a table |
| `-h`, `--help` | Full help |
| `-V`, `--version` | Print version |

With no argument and no `--file`, the sequence is read from stdin.

## Input handling

FASTA headers (lines starting with `>`) are skipped, and whitespace and digits
are ignored, so wrapped and numbered sequences work without cleaning them up
first. Input is case-insensitive.

Characters are sorted into three groups:

- the **20 standard amino acids**, which make up the main table
- **non-standard codes** — `B`, `Z`, `J`, `X`, `U`, `O`, `*`, `-` — reported
  separately and excluded from the percentages
- anything else, which is ignored with a warning on stderr

Warnings go to stderr, so `protcount --json | jq` stays clean.

Note that a multi-record FASTA file is treated as one concatenated sequence.

## Building from source

```sh
git clone https://github.com/Dipayan26/protcount
cd protcount
cargo build --release
cargo test
```

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

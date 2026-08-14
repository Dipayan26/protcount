//! protcount — count the amino acid composition of a protein sequence.
//!
//! Author: Dipayan Sarkar
//!
//! Reads a sequence from an argument, a file (plain or FASTA), or stdin,
//! and reports how many times each amino acid occurs.

use clap::Parser;
use std::collections::BTreeMap;
use std::io::Read;
use std::process::ExitCode;

// ---------------------------------------------------------------
// Reference table: one-letter code, three-letter code, full name.
//
// `const` data lives in the binary itself — no allocation, no setup.
// `&[(char, &str, &str)]` is a slice of 3-tuples.
// ---------------------------------------------------------------
const AMINO_ACIDS: &[(char, &str, &str)] = &[
    ('A', "Ala", "Alanine"),
    ('R', "Arg", "Arginine"),
    ('N', "Asn", "Asparagine"),
    ('D', "Asp", "Aspartic acid"),
    ('C', "Cys", "Cysteine"),
    ('Q', "Gln", "Glutamine"),
    ('E', "Glu", "Glutamic acid"),
    ('G', "Gly", "Glycine"),
    ('H', "His", "Histidine"),
    ('I', "Ile", "Isoleucine"),
    ('L', "Leu", "Leucine"),
    ('K', "Lys", "Lysine"),
    ('M', "Met", "Methionine"),
    ('F', "Phe", "Phenylalanine"),
    ('P', "Pro", "Proline"),
    ('S', "Ser", "Serine"),
    ('T', "Thr", "Threonine"),
    ('W', "Trp", "Tryptophan"),
    ('Y', "Tyr", "Tyrosine"),
    ('V', "Val", "Valine"),
];

/// Non-standard but legitimate sequence characters, reported separately.
const AMBIGUOUS: &[(char, &str)] = &[
    ('B', "Asn or Asp"),
    ('Z', "Gln or Glu"),
    ('J', "Leu or Ile"),
    ('X', "any / unknown"),
    ('U', "Selenocysteine"),
    ('O', "Pyrrolysine"),
    ('*', "stop codon"),
    ('-', "alignment gap"),
];

// ---------------------------------------------------------------
// Command line interface. clap turns this struct into the parser,
// the --help text, and the validation. Doc comments become help.
// ---------------------------------------------------------------
#[derive(Parser)]
#[command(
    name = "protcount",
    version,
    author = "Dipayan Sarkar",
    about = "Count the amino acid composition of a protein sequence",
    long_about = "Reads a protein sequence from an argument, a file, or stdin \
                  and reports the count and percentage of each amino acid.\n\n\
                  FASTA headers (lines starting with '>') are skipped, and \
                  whitespace and digits are ignored, so numbered or wrapped \
                  sequences work as-is."
)]
struct Args {
    /// Protein sequence given directly, e.g. MKTAYIAKQR
    #[arg(value_name = "SEQUENCE")]
    sequence: Option<String>,

    /// Read the sequence from a file instead (plain text or FASTA)
    #[arg(short, long, value_name = "PATH", conflicts_with = "sequence")]
    file: Option<String>,

    /// Sort output by count, highest first, instead of alphabetically
    #[arg(short, long)]
    sort: bool,

    /// Hide amino acids that do not occur in the sequence
    #[arg(short = 'z', long)]
    nonzero: bool,

    /// Emit JSON instead of a table
    #[arg(short, long)]
    json: bool,
}

// ---------------------------------------------------------------
// Core logic, kept free of any printing so it can be unit tested.
// ---------------------------------------------------------------

/// What a parsed sequence turned out to contain.
struct Composition {
    /// Counts for the 20 standard amino acids, keyed by one-letter code.
    /// BTreeMap keeps keys sorted, unlike HashMap.
    standard: BTreeMap<char, usize>,
    /// Counts for ambiguity codes, stops and gaps.
    ambiguous: BTreeMap<char, usize>,
    /// Characters that are not valid sequence characters at all.
    invalid: BTreeMap<char, usize>,
}

impl Composition {
    /// Total number of standard residues — the denominator for percentages.
    fn standard_total(&self) -> usize {
        self.standard.values().sum()
    }
}

/// Strip FASTA headers, whitespace and digits, and uppercase the rest.
fn clean_sequence(raw: &str) -> String {
    raw.lines()
        .filter(|line| !line.trim_start().starts_with('>'))
        .flat_map(|line| line.chars())
        .filter(|c| !c.is_whitespace() && !c.is_ascii_digit())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Tally each character of an already-cleaned sequence.
fn count_sequence(seq: &str) -> Composition {
    // Start every standard amino acid at zero so the table is complete.
    let mut standard: BTreeMap<char, usize> =
        AMINO_ACIDS.iter().map(|(code, _, _)| (*code, 0)).collect();
    let mut ambiguous = BTreeMap::new();
    let mut invalid = BTreeMap::new();

    for c in seq.chars() {
        if let Some(slot) = standard.get_mut(&c) {
            *slot += 1;
        } else if AMBIGUOUS.iter().any(|(code, _)| *code == c) {
            *ambiguous.entry(c).or_insert(0) += 1;
        } else {
            *invalid.entry(c).or_insert(0) += 1;
        }
    }

    Composition {
        standard,
        ambiguous,
        invalid,
    }
}

/// Look up the three-letter code and full name for a one-letter code.
fn describe(code: char) -> (&'static str, &'static str) {
    AMINO_ACIDS
        .iter()
        .find(|(c, _, _)| *c == code)
        .map(|(_, three, name)| (*three, *name))
        .unwrap_or(("???", "unknown"))
}

// ---------------------------------------------------------------
// Input handling and output rendering.
// ---------------------------------------------------------------

/// Resolve where the sequence text comes from: argument, file, or stdin.
fn read_input(args: &Args) -> Result<String, String> {
    if let Some(seq) = &args.sequence {
        return Ok(seq.clone());
    }
    if let Some(path) = &args.file {
        // `?` on a std::io::Error needs converting to our String error type.
        return std::fs::read_to_string(path).map_err(|e| format!("could not read {path}: {e}"));
    }

    // Nothing given: fall back to stdin so the tool can be piped into.
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| format!("could not read stdin: {e}"))?;

    if buf.trim().is_empty() {
        return Err(
            "no sequence given (pass one as an argument, use --file, or pipe it in)".to_string(),
        );
    }
    Ok(buf)
}

fn print_table(comp: &Composition, sort: bool, nonzero: bool) {
    let total = comp.standard_total();

    // Collect into a Vec so it can be reordered; BTreeMap is always sorted.
    let mut rows: Vec<(char, usize)> = comp
        .standard
        .iter()
        .map(|(code, count)| (*code, *count))
        .filter(|(_, count)| !nonzero || *count > 0)
        .collect();

    if sort {
        // Sort by count descending, breaking ties by letter for stability.
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    }

    println!("Standard residues: {total}");
    println!();
    println!(
        "{:<2} {:<4} {:<14} {:>6} {:>7}",
        "", "3AA", "Name", "Count", "Pct"
    );
    println!("{}", "-".repeat(50));

    for (code, count) in &rows {
        let (three, name) = describe(*code);
        let pct = if total > 0 {
            (*count as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        // A crude bar: one block per 2% of the sequence.
        let bar = "#".repeat((pct / 2.0).round() as usize);
        println!("{code:<2} {three:<4} {name:<14} {count:>6} {pct:>6.2}%  {bar}");
    }

    if !comp.ambiguous.is_empty() {
        println!();
        println!("Non-standard characters:");
        for (code, count) in &comp.ambiguous {
            let meaning = AMBIGUOUS
                .iter()
                .find(|(c, _)| c == code)
                .map(|(_, m)| *m)
                .unwrap_or("?");
            println!("  {code}  {count:>6}  ({meaning})");
        }
    }
}

fn print_json(comp: &Composition) {
    // Built by hand to keep the dependency list to just clap.
    let total = comp.standard_total();
    println!("{{");
    println!("  \"standard_total\": {total},");
    println!("  \"counts\": {{");

    let entries: Vec<String> = comp
        .standard
        .iter()
        .map(|(code, count)| format!("    \"{code}\": {count}"))
        .collect();
    println!("{}", entries.join(",\n"));
    println!("  }},");

    let amb: Vec<String> = comp
        .ambiguous
        .iter()
        .map(|(code, count)| format!("    \"{code}\": {count}"))
        .collect();
    println!("  \"non_standard\": {{");
    if !amb.is_empty() {
        println!("{}", amb.join(",\n"));
    }
    println!("  }}");
    println!("}}");
}

/// The real main. Returning Result lets every failure path use `?`.
fn run(args: &Args) -> Result<(), String> {
    let raw = read_input(args)?;
    let seq = clean_sequence(&raw);

    if seq.is_empty() {
        return Err("sequence is empty after removing headers and whitespace".to_string());
    }

    let comp = count_sequence(&seq);

    // Warn about junk on stderr so it never pollutes piped stdout.
    if !comp.invalid.is_empty() {
        let list: Vec<String> = comp
            .invalid
            .iter()
            .map(|(c, n)| format!("{c:?} x{n}"))
            .collect();
        eprintln!(
            "warning: ignored {} invalid character(s): {}",
            comp.invalid.values().sum::<usize>(),
            list.join(", ")
        );
    }

    if args.json {
        print_json(&comp);
    } else {
        print_table(&comp, args.sort, args.nonzero);
    }
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("protcount: {msg}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------
// Tests — run with `cargo test`
// ---------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_simple_sequence() {
        let comp = count_sequence(&clean_sequence("AAAC"));
        assert_eq!(comp.standard[&'A'], 3);
        assert_eq!(comp.standard[&'C'], 1);
        assert_eq!(comp.standard[&'W'], 0);
        assert_eq!(comp.standard_total(), 4);
    }

    #[test]
    fn is_case_insensitive() {
        let comp = count_sequence(&clean_sequence("aAaA"));
        assert_eq!(comp.standard[&'A'], 4);
    }

    #[test]
    fn strips_fasta_header_and_whitespace() {
        let fasta = ">sp|P12345|TEST_HUMAN Some protein\nMKTA\nYIAK\n";
        let seq = clean_sequence(fasta);
        assert_eq!(seq, "MKTAYIAK");
        assert_eq!(count_sequence(&seq).standard_total(), 8);
    }

    #[test]
    fn strips_line_numbers() {
        assert_eq!(clean_sequence("1 MKTA 5 YIAK"), "MKTAYIAK");
    }

    #[test]
    fn separates_ambiguous_from_invalid() {
        let comp = count_sequence(&clean_sequence("ACXZ!!"));
        assert_eq!(comp.standard_total(), 2);
        assert_eq!(comp.ambiguous[&'X'], 1);
        assert_eq!(comp.ambiguous[&'Z'], 1);
        assert_eq!(comp.invalid[&'!'], 2);
    }

    #[test]
    fn empty_sequence_has_zero_total() {
        let comp = count_sequence("");
        assert_eq!(comp.standard_total(), 0);
        assert_eq!(comp.standard.len(), 20);
    }
}

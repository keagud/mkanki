use clap::Parser;
use itertools::Itertools;
use mkanki::cli::{Cli, CONFIG_FILE};
use mkanki::mkanki::{make_deck_name, read_config, read_multiple_md};

fn main() -> mkanki::Result<()> {
    let mut cli = Cli::parse();

    let config_file = cli.config.take().unwrap_or_else(|| CONFIG_FILE.clone());
    let configured_decks = read_config(&config_file)?;

    let selected_deck = if let Some(cli_deck_choice) = cli.deck {
        let deck_opts = configured_decks
            .into_iter()
            .filter(|d| {
                d.name
                    .to_lowercase()
                    .starts_with(&cli_deck_choice.to_lowercase())
            })
            .collect_vec();

        if deck_opts.len() > 1 {
            let opts = deck_opts
                .iter()
                .map(|x| format!("'{}'", &x.name))
                .join(", ");
            return Err(format!(
                "Multiple matching decks for '{}', cannot disambiguate between:\n{}",
                &cli_deck_choice, opts
            )
            .into());
        }

        match deck_opts.into_iter().next() {
            Some(d) => d,
            None => {
                return Err(format!("No deck matched '{}'", &cli_deck_choice).into());
            }
        }
    } else {
        configured_decks
            .into_iter()
            .find(|d| d.is_default)
            .ok_or("Error parsing config file: no default deck found")?
    };

    let parsed_notes = read_multiple_md(&cli.input)?
        .into_iter()
        .map(|n| n.to_note(&selected_deck))
        .collect::<Result<Vec<_>, _>>()?;

    let mut deck = selected_deck.as_deck();

    for note in parsed_notes.into_iter() {
        deck.add_note(note);
    }

    let output_deck_file = match cli.output {
        Some(f) => f.canonicalize()?,
        None => std::env::current_dir()?.join(make_deck_name(&selected_deck.name)),
    };

    deck.write_to_file(&output_deck_file.to_string_lossy())?;

    Ok(())
}

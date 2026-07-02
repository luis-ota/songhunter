pub mod acoustid;
pub mod acrcloud;
pub mod backend;
pub mod registry;
pub mod shazam;

pub use backend::Identifier;
pub use registry::IdentifierRegistry;

/// Junta resultados de múltiplos provedores, remove duplicatas e ranqueia.
pub fn merge_results(
    results: Vec<(String, Vec<crate::models::SongMatch>)>,
) -> Vec<crate::models::SongMatch> {
    use std::collections::HashSet;

    let mut combined: Vec<crate::models::SongMatch> = results
        .into_iter()
        .flat_map(|(provider, songs)| {
            songs.into_iter().map(move |mut s| {
                s.provider = provider.clone();
                s
            })
        })
        .collect();

    combined.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for song in combined {
        let key = format!(
            "{} - {}",
            song.artist.to_lowercase().trim(),
            song.title.to_lowercase().trim()
        );
        if seen.insert(key) {
            deduped.push(song);
        }
    }

    for (i, song) in deduped.iter_mut().enumerate() {
        song.rank = Some(i + 1);
    }

    deduped.into_iter().take(4).collect()
}

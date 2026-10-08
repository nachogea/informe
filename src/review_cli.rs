//! Local CLI adapter: no provider credentials, shell actions, or model calls.
use crate::{
    ipc,
    review::{self, Target},
};
use std::{
    ffi::OsString,
    path::PathBuf,
    time::{Duration, Instant},
};
pub fn run(args: &[OsString]) -> Option<Result<(), String>> {
    let command = args.first()?.to_str()?;
    if !["comment", "submit", "review", "apply-revision"].contains(&command) {
        return None;
    }
    Some(execute(command, &args[1..]))
}
fn execute(command: &str, args: &[OsString]) -> Result<(), String> {
    let path = PathBuf::from(
        args.first()
            .ok_or("Review command requires an artifact path")?,
    );
    if command == "apply-revision" {
        if args.len() != 4 || args[2] != "--expect" {
            return Err(
                "Usage: apply-revision artifact.json revised.json --expect CONTENT_HASH".into(),
            );
        }
        review::apply_revision(&path, &PathBuf::from(&args[1]), &args[3].to_string_lossy())?;
        println!("Applied revision to {}", path.display());
        return Ok(());
    }
    let mut node = None;
    let mut quote = None;
    let mut text = None;
    let mut offset = None;
    let mut wait = false;
    let mut after = None;
    let mut timeout = 300u64;
    let mut timeout_given = false;
    let mut i = 1;
    while i < args.len() {
        let flag = args[i].to_str().ok_or("Invalid option")?;
        if flag == "--wait" {
            wait = true;
            i += 1;
            continue;
        }
        let value = args
            .get(i + 1)
            .ok_or("Option requires a value")?
            .to_string_lossy()
            .into_owned();
        match flag {
            "--node" => node = Some(value),
            "--quote" => quote = Some(value),
            "--text" => text = Some(value),
            "--offset" => offset = Some(value.parse::<usize>().map_err(|_| "Invalid byte offset")?),
            "--after" => after = Some(value),
            "--timeout" => {
                timeout = value.parse().map_err(|_| "Invalid timeout")?;
                timeout_given = true;
            }
            _ => return Err(format!("Unknown review option {flag}")),
        }
        i += 2;
    }
    match command {
        "comment" => {
            if wait || after.is_some() || timeout_given {
                return Err("Wait/after/timeout options apply only to review".into());
            }
            if offset.is_some() && quote.is_none() {
                return Err("--offset requires --quote".into());
            }
            let body = text.ok_or("comment requires --text")?;
            let (_, id) = review::edit(&path, None, |state, artifact| {
                let target = if let Some(quote) = quote {
                    Target::text(artifact, node.as_deref(), &quote, offset)?
                } else {
                    Target::node(
                        artifact,
                        node.as_deref()
                            .ok_or("comment requires --node or --quote")?,
                    )?
                };
                state.add(artifact, target, &body)
            })?;
            println!("{id}");
        }
        "submit" => {
            if args.len() != 1 {
                return Err("Usage: submit artifact.json".into());
            }
            let (_, feedback) = review::edit(&path, None, |state, a| state.submit(a, &path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&feedback).map_err(|e| e.to_string())?
            );
        }
        "review" => {
            if node.is_some() || quote.is_some() || text.is_some() || offset.is_some() {
                return Err("review accepts only --wait, --after and --timeout".into());
            }
            if timeout == 0 || timeout > 86400 {
                return Err("Timeout must be 1–86400 seconds".into());
            }
            let a = review::read_artifact(&path)?;
            let initial = review::load(&path, &a)?;
            let baseline = after.or_else(|| {
                wait.then(|| initial.submission.as_ref().map(|s| s.id.clone()))
                    .flatten()
            });
            if wait {
                ipc::open(&path)?;
            }
            let deadline = Instant::now() + Duration::from_secs(timeout);
            loop {
                let a = review::read_artifact(&path)?;
                let state = review::load(&path, &a)?;
                if let Some(batch) = state.submission
                    && (!wait || baseline.as_deref() != Some(&batch.id))
                {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&batch).map_err(|e| e.to_string())?
                    );
                    return Ok(());
                }
                if !wait {
                    return Err(
                        "No submitted feedback. Add comments and choose Send feedback in the app."
                            .into(),
                    );
                }
                if Instant::now() >= deadline {
                    return Err("Timed out waiting for submitted feedback".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

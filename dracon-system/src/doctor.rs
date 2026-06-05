//! System diagnostics — deterministic health checks for canonical dracon setup.

use anyhow::Result;
use std::path::PathBuf;

use crate::{canonical_system_root, is_user_service_active};

/// Run the diagnostic check and return a report.
pub(crate) async fn build_doctor_report() -> crate::DoctorReport {
    let root = canonical_system_root();
    let nixos = root.join("nixos");
    let libs = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join("Dev/dracon-libs");
    let utils = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join("Dev/dracon-utilities");
    let policy = root.join("utilities/sync/dracon-sync.toml");
    let legacy_cfg = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join(".config/dracon");

    crate::DoctorReport {
        system_root_exists: root.exists(),
        nixos_root_exists: nixos.exists(),
        canonical_libs_exists: libs.exists(),
        canonical_utils_exists: utils.exists(),
        sync_policy_exists: policy.exists(),
        legacy_config_dracon_exists: legacy_cfg.exists(),
        sync_service_active: is_user_service_active("dracon-sync.service").await,
    }
}

/// Handle the `doctor` CLI subcommand.
pub(crate) async fn cmd_doctor(json: bool, strict: bool) -> Result<()> {
    use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

    let report = build_doctor_report().await;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        if strict {
            // In strict mode, fail if any check fails
            if !report.all_ok() {
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new(" "),
            Cell::new("CHECK"),
            Cell::new("STATUS"),
        ]);

    let checks: Vec<(&str, bool)> = vec![
        ("~/.dracon/nixos", report.nixos_root_exists),
        ("dracon-libs (sibling)", report.canonical_libs_exists),
        ("dracon-utilities (self)", report.canonical_utils_exists),
        ("sync policy", report.sync_policy_exists),
        ("legacy config", !report.legacy_config_dracon_exists),
        ("sync service", report.sync_service_active),
    ];

    let mut has_failures = false;
    for (label, ok) in &checks {
        let (icon, color) = if *ok {
            ("\u{2705}", Color::Green)
        } else {
            ("\u{274c}", Color::Red)
        };
        if !ok {
            has_failures = true;
        }
        table.add_row(vec![
            Cell::new(icon).fg(color),
            Cell::new(*label),
            Cell::new(if *ok { "ok" } else { "missing" }),
        ]);
    }

    println!("{table}");
    if has_failures {
        eprintln!("\n\u{26a0}\u{fe0f} Some checks failed. Run with --json for details.");
        if strict {
            std::process::exit(1);
        }
    }
    Ok(())
}

use std::any::Any;
use std::cell::RefCell;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{mpsc, Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use org_eclipse_elk_alg_disco::org::eclipse::elk::alg::disco::options::disco_meta_data_provider::DisCoMetaDataProvider;
use org_eclipse_elk_alg_disco::org::eclipse::elk::alg::disco::DisCoLayoutProvider;
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::force_layout_provider::ForceLayoutProvider;
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::options::force_meta_data_provider::ForceMetaDataProvider;
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::options::stress_meta_data_provider::StressMetaDataProvider;
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::stress::stress_layout_provider::StressLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredMetaDataProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_alg_mrtree::org::eclipse::elk::alg::mrtree::options::mrtree_meta_data_provider::MrTreeMetaDataProvider;
use org_eclipse_elk_alg_mrtree::org::eclipse::elk::alg::mrtree::tree_layout_provider::TreeLayoutProvider;
use org_eclipse_elk_alg_radial::org::eclipse::elk::alg::radial::options::radial_meta_data_provider::RadialMetaDataProvider;
use org_eclipse_elk_alg_radial::org::eclipse::elk::alg::radial::radial_layout_provider::RadialLayoutProvider;
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::options::rect_packing_meta_data_provider::RectPackingMetaDataProvider;
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::rect_packing_layout_provider::RectPackingLayoutProvider;
use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::options::spore_meta_data_provider::SporeMetaDataProvider;
use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::overlap_removal_layout_provider::OverlapRemovalLayoutProvider;
use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::shrink_tree_layout_provider::ShrinkTreeLayoutProvider;
use org_eclipse_elk_alg_vertiflex::org::eclipse::elk::alg::vertiflex::options::vertiflex_meta_data_provider::VertiFlexMetaDataProvider;
use org_eclipse_elk_alg_vertiflex::org::eclipse::elk::alg::vertiflex::vertiflex_layout_provider::VertiFlexLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    AlgorithmFactory, BasicProgressMonitor, InstancePool, Maybe,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::{ElkGraphJson, JsonImporter};

const HEADER_JAVA: &str = "model_rel_path\tinput_json\tjava_layout_json\tjava_status\tjava_error";
const HEADER_RUST: &str = "model_rel_path\tinput_json\tjava_layout_json\tjava_status\tjava_error\trust_layout_json\trust_status\trust_error";

#[derive(Debug)]
struct Config {
    input_manifest: PathBuf,
    output_manifest: PathBuf,
    rust_layout_dir: PathBuf,
    pretty_print: bool,
    stop_on_error: bool,
}

#[derive(Debug)]
struct JavaManifestRow {
    model_rel_path: String,
    input_json: String,
    java_layout_json: String,
    java_status: String,
    java_error: String,
}

fn print_usage() {
    println!(
        "Usage:
  cargo run -p org-eclipse-elk-graph-json --bin model_parity_layout_runner -- \\
    --input-manifest <path> \\
    --output-manifest <path> \\
    --rust-layout-dir <path> \\
    [--pretty-print <true|false>] \\
    [--stop-on-error <true|false>]"
    );
}

fn parse_bool_flag(value: &str, flag: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" => Ok(true),
        "0" | "false" | "no" | "n" => Ok(false),
        _ => Err(format!("invalid value for {flag}: {value}")),
    }
}

fn parse_args() -> Result<Config, String> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        std::process::exit(0);
    }

    let mut input_manifest: Option<PathBuf> = None;
    let mut output_manifest: Option<PathBuf> = None;
    let mut rust_layout_dir: Option<PathBuf> = None;
    let mut pretty_print = false;
    let mut stop_on_error = false;

    let mut index = 1usize;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--input-manifest" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for --input-manifest".to_string())?;
                input_manifest = Some(PathBuf::from(value));
            }
            "--output-manifest" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for --output-manifest".to_string())?;
                output_manifest = Some(PathBuf::from(value));
            }
            "--rust-layout-dir" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for --rust-layout-dir".to_string())?;
                rust_layout_dir = Some(PathBuf::from(value));
            }
            "--pretty-print" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for --pretty-print".to_string())?;
                pretty_print = parse_bool_flag(value, "--pretty-print")?;
            }
            "--stop-on-error" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for --stop-on-error".to_string())?;
                stop_on_error = parse_bool_flag(value, "--stop-on-error")?;
            }
            _ => {
                return Err(format!("unknown argument: {flag}"));
            }
        }
        index += 1;
    }

    let input_manifest = input_manifest.ok_or_else(|| "missing --input-manifest".to_string())?;
    let output_manifest = output_manifest.ok_or_else(|| "missing --output-manifest".to_string())?;
    let rust_layout_dir = rust_layout_dir.ok_or_else(|| "missing --rust-layout-dir".to_string())?;

    Ok(Config {
        input_manifest,
        output_manifest,
        rust_layout_dir,
        pretty_print,
        stop_on_error,
    })
}

fn sanitize_tsv(value: &str) -> String {
    value.replace(['\t', '\r', '\n'], " ")
}

fn read_java_manifest(path: &Path) -> Result<Vec<JavaManifestRow>, String> {
    let file = File::open(path)
        .map_err(|err| format!("failed to open java manifest {}: {err}", path.display()))?;
    let reader = BufReader::new(file);
    let mut rows = Vec::new();

    for (line_no, line_result) in reader.lines().enumerate() {
        let line = line_result.map_err(|err| {
            format!(
                "failed to read java manifest {} line {}: {err}",
                path.display(),
                line_no + 1
            )
        })?;

        if line_no == 0 {
            let header = line.trim_start_matches('\u{feff}');
            if header != HEADER_JAVA {
                return Err(format!(
                    "unexpected java manifest header in {}: expected `{HEADER_JAVA}`, got `{header}`",
                    path.display()
                ));
            }
            continue;
        }

        if line.trim().is_empty() {
            continue;
        }

        let columns: Vec<&str> = line.splitn(5, '\t').collect();
        if columns.len() != 5 {
            return Err(format!(
                "invalid java manifest row at {} line {}",
                path.display(),
                line_no + 1
            ));
        }

        rows.push(JavaManifestRow {
            model_rel_path: columns[0].to_string(),
            input_json: columns[1].to_string(),
            java_layout_json: columns[2].to_string(),
            java_status: columns[3].to_string(),
            java_error: columns[4].to_string(),
        });
    }

    if rows.is_empty() {
        return Err(format!(
            "java manifest has no data rows: {}",
            path.display()
        ));
    }

    Ok(rows)
}

fn provider_pool<F>(creator: F) -> Arc<InstancePool<Box<dyn AbstractLayoutProvider>>>
where
    F: Fn() -> Box<dyn AbstractLayoutProvider> + Send + Sync + 'static,
{
    Arc::new(InstancePool::new(Box::new(AlgorithmFactory::new(creator))))
}

fn initialize_plain_java_like_layout() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        initialize_plain_java_layout();

        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
        service.register_layout_meta_data_provider(&DisCoMetaDataProvider);
        service.register_layout_meta_data_provider(&ForceMetaDataProvider);
        service.register_layout_meta_data_provider(&MrTreeMetaDataProvider);
        service.register_layout_meta_data_provider(&RadialMetaDataProvider);
        service.register_layout_meta_data_provider(&RectPackingMetaDataProvider);
        service.register_layout_meta_data_provider(&SporeMetaDataProvider);
        service.register_layout_meta_data_provider(&StressMetaDataProvider);
        service.register_layout_meta_data_provider(&VertiFlexMetaDataProvider);

        service.override_algorithm_provider_pool(
            "org.eclipse.elk.layered",
            provider_pool(|| Box::new(LayeredLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.disco",
            provider_pool(|| Box::new(DisCoLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.force",
            provider_pool(|| Box::new(ForceLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.stress",
            provider_pool(|| Box::new(StressLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.mrtree",
            provider_pool(|| Box::new(TreeLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.radial",
            provider_pool(|| Box::new(RadialLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.rectpacking",
            provider_pool(|| Box::new(RectPackingLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.sporeOverlap",
            provider_pool(|| Box::new(OverlapRemovalLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.sporeCompaction",
            provider_pool(|| Box::new(ShrinkTreeLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.vertiflex",
            provider_pool(|| Box::new(VertiFlexLayoutProvider::new())),
        );
    });
}

fn panic_payload_to_string(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "unknown panic payload".to_string()
}

fn run_layout_case(
    input_json_path: &Path,
    output_json_path: &Path,
    pretty_print: bool,
    random_seed_override: Option<i32>,
) -> Result<(), String> {
    let result = panic::catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        let input_text = fs::read_to_string(input_json_path)
            .map_err(|err| format!("failed to read input json {}: {err}", input_json_path.display()))?;
        let input_value: Value = serde_json::from_str(&input_text)
            .map_err(|err| format!("failed to parse input json {}: {err}", input_json_path.display()))?;

        let shared = Rc::new(RefCell::new(input_value));
        let mut importer_slot: Maybe<JsonImporter> = Maybe::default();
        let root = ElkGraphJson::for_graph_shared(shared.clone())
            .remember_importer(&mut importer_slot)
            .lenient(false)
            .to_elk()
            .map_err(|err| format!("ELK graph import failed: {err}"))?;

        if let Some(seed) = random_seed_override {
            root.borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .set_property(CoreOptions::RANDOM_SEED, Some(seed));
        }

        if std::env::var_os("ELK_TRACE_PRE_LAYOUT_PORT_ORDER").is_some() {
            trace_port_order_before_layout(&root);
        }

        let mut engine = RecursiveGraphLayoutEngine::new();
        let mut monitor = BasicProgressMonitor::new();
        engine.layout(&root, &mut monitor);

        let importer = importer_slot
            .get_mut()
            .ok_or_else(|| "internal error: missing json importer slot".to_string())?;
        importer
            .transfer_layout(&root)
            .map_err(|err| format!("layout transfer failed: {err}"))?;

        if let Some(parent) = output_json_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create output directory {}: {err}",
                    parent.display()
                )
            })?;
        }

        let serialized = if pretty_print {
            serde_json::to_string_pretty(&*shared.borrow())
                .map_err(|err| format!("failed to serialize pretty JSON: {err}"))?
        } else {
            serde_json::to_string(&*shared.borrow())
                .map_err(|err| format!("failed to serialize JSON: {err}"))?
        };

        fs::write(output_json_path, serialized).map_err(|err| {
            format!(
                "failed to write output json {}: {err}",
                output_json_path.display()
            )
        })?;

        Ok(())
    }));

    match result {
        Ok(inner) => inner,
        Err(payload) => Err(format!("panic during layout: {}", panic_payload_to_string(payload.as_ref()))),
    }
}

fn trace_port_order_before_layout(root: &ElkNodeRef) {
    let mut stack = vec![root.clone()];
    while let Some(node) = stack.pop() {
        let (node_id, port_ids, children) = {
            let mut node_mut = node.borrow_mut();
            let node_id = node_mut
                .connectable()
                .shape()
                .graph_element()
                .identifier()
                .unwrap_or("<no-node-id>")
                .to_owned();
            let port_ids = node_mut
                .ports()
                .iter()
                .map(|port| {
                    port.borrow_mut()
                        .connectable()
                        .shape()
                        .graph_element()
                        .identifier()
                        .unwrap_or("<no-port-id>")
                        .to_owned()
                })
                .collect::<Vec<_>>()
                .join(", ");
            let children = node_mut.children().iter().cloned().collect::<Vec<_>>();
            (node_id, port_ids, children)
        };
        eprintln!(
            "rust-pre-layout-port-order: node={} ports=[{}]",
            node_id, port_ids
        );
        for child in children {
            stack.push(child);
        }
    }
}

fn parse_random_seed_override() -> Result<Option<i32>, String> {
    let value = match env::var("MODEL_PARITY_RANDOM_SEED") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => return Ok(None),
        Err(err) => {
            return Err(format!(
                "failed to read MODEL_PARITY_RANDOM_SEED environment variable: {err}"
            ));
        }
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let seed = trimmed.parse::<i32>().map_err(|err| {
        format!(
            "invalid MODEL_PARITY_RANDOM_SEED value `{trimmed}` (expected integer): {err}"
        )
    })?;
    Ok(Some(seed))
}

fn parse_timeout_secs() -> Result<u64, String> {
    let value = match env::var("MODEL_PARITY_TIMEOUT_SECS") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => return Ok(120),
        Err(err) => {
            return Err(format!(
                "failed to read MODEL_PARITY_TIMEOUT_SECS environment variable: {err}"
            ));
        }
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(120);
    }
    trimmed.parse::<u64>().map_err(|err| {
        format!("invalid MODEL_PARITY_TIMEOUT_SECS value `{trimmed}` (expected positive integer): {err}")
    })
}

fn run_layout_case_with_timeout(
    input_json_path: PathBuf,
    output_json_path: PathBuf,
    pretty_print: bool,
    random_seed_override: Option<i32>,
    timeout: Duration,
) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    let _handle = thread::spawn(move || {
        let result = run_layout_case(&input_json_path, &output_json_path, pretty_print, random_seed_override);
        let _ = tx.send(result);
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err("timeout".to_string()),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("layout thread disconnected unexpectedly".to_string()),
    }
}

fn run(config: Config) -> Result<(), String> {
    initialize_plain_java_like_layout();
    let random_seed_override = parse_random_seed_override()?;
    let timeout_secs = parse_timeout_secs()?;
    let timeout = Duration::from_secs(timeout_secs);

    let java_rows = read_java_manifest(&config.input_manifest)?;
    let total_count = java_rows.len();
    eprintln!("model parity rust runner: total={total_count}, timeout={timeout_secs}s");

    if let Some(parent) = config.output_manifest.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create output manifest directory {}: {err}",
                parent.display()
            )
        })?;
    }
    fs::create_dir_all(&config.rust_layout_dir).map_err(|err| {
        format!(
            "failed to create rust layout directory {}: {err}",
            config.rust_layout_dir.display()
        )
    })?;

    let manifest_file = File::create(&config.output_manifest).map_err(|err| {
        format!(
            "failed to create output manifest {}: {err}",
            config.output_manifest.display()
        )
    })?;
    let mut writer = BufWriter::new(manifest_file);
    writeln!(writer, "{HEADER_RUST}")
        .map_err(|err| format!("failed to write output manifest header: {err}"))?;

    let mut total = 0usize;
    let mut ok = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;
    let mut timeouts = 0usize;

    for row in java_rows {
        total += 1;
        let model_start = Instant::now();

        let (rust_layout_json, rust_status, rust_error) = if row.java_status != "ok" {
            skipped += 1;
            (
                String::new(),
                "skipped_java_non_ok".to_string(),
                format!("java status was `{}`", row.java_status),
            )
        } else {
            let output_json_path = config
                .rust_layout_dir
                .join(format!("{}.json", row.model_rel_path));
            let rust_layout_json = output_json_path.display().to_string();

            match run_layout_case_with_timeout(
                Path::new(&row.input_json).to_path_buf(),
                output_json_path,
                config.pretty_print,
                random_seed_override,
                timeout,
            ) {
                Ok(()) => {
                    ok += 1;
                    (rust_layout_json, "ok".to_string(), String::new())
                }
                Err(err) if err == "timeout" => {
                    timeouts += 1;
                    errors += 1;
                    (rust_layout_json, "timeout".to_string(), format!("exceeded {timeout_secs} seconds"))
                }
                Err(err) => {
                    errors += 1;
                    (rust_layout_json, "error".to_string(), err)
                }
            }
        };

        let elapsed_ms = model_start.elapsed().as_millis();
        eprintln!("[{total}/{total_count}] {rust_status} {} ({elapsed_ms} ms)", row.model_rel_path);

        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            sanitize_tsv(&row.model_rel_path),
            sanitize_tsv(&row.input_json),
            sanitize_tsv(&row.java_layout_json),
            sanitize_tsv(&row.java_status),
            sanitize_tsv(&row.java_error),
            sanitize_tsv(&rust_layout_json),
            sanitize_tsv(&rust_status),
            sanitize_tsv(&rust_error),
        )
        .map_err(|err| format!("failed to write output manifest row: {err}"))?;

        // Flush after each row so partial results survive if the process is killed
        writer
            .flush()
            .map_err(|err| format!("failed to flush output manifest: {err}"))?;

        if config.stop_on_error && rust_status == "error" {
            return Err(format!(
                "stopped on first rust layout error (model `{}`): {}",
                row.model_rel_path, rust_error
            ));
        }
    }

    eprintln!(
        "model parity rust runner completed: total={total}, ok={ok}, skipped={skipped}, errors={errors} (timeouts={timeouts})"
    );

    Ok(())
}

fn main() {
    let config = match parse_args() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            print_usage();
            std::process::exit(2);
        }
    };

    if let Err(err) = run(config) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

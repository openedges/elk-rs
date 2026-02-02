use std::cell::Cell;
use std::env;
use std::rc::Rc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use org_eclipse_elk_core::org::eclipse::elk::core::comments::{
    CacheKey, CommentAttacher, IDataProvider, IMatcher, NodeReferenceMatcher,
};

#[derive(Clone)]
struct Comment {
    id: usize,
    text: String,
}

#[derive(Clone)]
struct Target {
    id: usize,
    name: String,
}

impl CacheKey for Comment {
    fn cache_key(&self) -> usize {
        self.id
    }
}

impl CacheKey for Target {
    fn cache_key(&self) -> usize {
        self.id
    }
}

struct PerfDataProvider {
    comments: Vec<Comment>,
    targets: Vec<Target>,
    attachments: Cell<usize>,
}

impl PerfDataProvider {
    fn new(count: usize) -> Self {
        let mut comments = Vec::with_capacity(count);
        let mut targets = Vec::with_capacity(count);
        for i in 0..count {
            comments.push(Comment {
                id: i,
                text: format!("Node{i}"),
            });
            targets.push(Target {
                id: i,
                name: format!("Node{i}"),
            });
        }
        PerfDataProvider {
            comments,
            targets,
            attachments: Cell::new(0),
        }
    }

    fn reset(&self) {
        self.attachments.set(0);
    }
}

impl IDataProvider<Comment, Target> for PerfDataProvider {
    fn provide_comments(&self) -> Vec<Comment> {
        self.comments.clone()
    }

    fn provide_targets(&self) -> Vec<Target> {
        self.targets.clone()
    }

    fn provide_sub_hierarchies(&self) -> Vec<Rc<dyn IDataProvider<Comment, Target>>> {
        Vec::new()
    }

    fn attach(&self, _comment: &Comment, _target: &Target) {
        self.attachments.set(self.attachments.get() + 1);
    }
}

fn parse_arg(args: &[String], flag: &str, default: usize) -> usize {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn parse_arg_str(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(|value| value.to_string())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let count = parse_arg(&args, "--count", 2_000);
    let iterations = parse_arg(&args, "--iterations", 5);
    let warmup = parse_arg(&args, "--warmup", 1);
    let output_path = parse_arg_str(&args, "--output");

    let provider = PerfDataProvider::new(count);

    let mut matcher = NodeReferenceMatcher::new();
    matcher
        .with_comment_text_provider(|c: &Comment| c.text.clone())
        .with_target_name_provider(|t: &Target| t.name.clone());

    let mut attacher = CommentAttacher::new();
    attacher.add_matcher(Rc::new(matcher) as Rc<dyn IMatcher<Comment, Target>>);

    for _ in 0..warmup {
        attacher.attach_comments(&provider);
        provider.reset();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        attacher.attach_comments(&provider);
        provider.reset();
    }
    let elapsed = start.elapsed();

    let total_ops = count.saturating_mul(iterations);
    let nanos = elapsed.as_nanos().max(1) as f64;
    let ops_per_sec = (total_ops as f64) / (nanos / 1_000_000_000.0);
    let avg_nanos = nanos / iterations.max(1) as f64;

    println!(
        "Comment attachment: {count} items, {iterations} iterations, warmup {warmup} -> {:?}",
        elapsed
    );
    println!(
        "Average per iteration: {:.2} ms, throughput: {:.2} ops/s",
        avg_nanos / 1_000_000.0,
        ops_per_sec
    );

    if let Some(path) = output_path {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let line = format!(
            "{timestamp},{count},{iterations},{warmup},{},{:.6},{:.2}\n",
            elapsed.as_nanos(),
            avg_nanos / 1_000_000.0,
            ops_per_sec
        );
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            use std::io::Write;
            let _ = file.write_all(line.as_bytes());
        }
    }
}

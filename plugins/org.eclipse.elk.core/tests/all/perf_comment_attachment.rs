use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

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
                text: format!("Node{}", i),
            });
            targets.push(Target {
                id: i,
                name: format!("Node{}", i),
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

    #[allow(dead_code)]
    fn attachment_count(&self) -> usize {
        self.attachments.get()
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

#[ignore]
#[test]
fn perf_comment_attachment_node_reference() {
    let count = 2_000;
    let iterations = 5;

    let provider = PerfDataProvider::new(count);

    let mut matcher = NodeReferenceMatcher::new();
    matcher
        .with_comment_text_provider(|c: &Comment| c.text.clone())
        .with_target_name_provider(|t: &Target| t.name.clone());

    let mut attacher = CommentAttacher::new();
    attacher.add_matcher(Rc::new(matcher) as Rc<dyn IMatcher<Comment, Target>>);

    // Warm-up
    attacher.attach_comments(&provider);
    provider.reset();

    let start = Instant::now();
    for _ in 0..iterations {
        attacher.attach_comments(&provider);
        provider.reset();
    }
    let elapsed = start.elapsed();

    println!(
        "Comment attachment: {} comments/targets, {} iterations -> {:?}",
        count, iterations, elapsed
    );
}

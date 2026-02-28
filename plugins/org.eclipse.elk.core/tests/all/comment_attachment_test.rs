use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::comments::{
    CacheKey, CommentAttacher, IDataProvider, IExplicitAttachmentProvider, NodeReferenceMatcher,
    TextPrefixFilter,
};
#[derive(Clone, Debug)]
struct Comment {
    id: usize,
    text: String,
}

#[derive(Clone, Debug)]
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

struct SimpleDataProvider {
    comments: Vec<Comment>,
    targets: Vec<Target>,
    attachments: RefCell<Vec<(usize, usize)>>,
}

impl SimpleDataProvider {
    fn new(comments: Vec<Comment>, targets: Vec<Target>) -> Self {
        SimpleDataProvider {
            comments,
            targets,
            attachments: RefCell::new(Vec::new()),
        }
    }

    fn attachments(&self) -> Vec<(usize, usize)> {
        self.attachments.borrow().clone()
    }
}

impl IDataProvider<Comment, Target> for SimpleDataProvider {
    fn provide_comments(&self) -> Vec<Comment> {
        self.comments.clone()
    }

    fn provide_targets(&self) -> Vec<Target> {
        self.targets.clone()
    }

    fn provide_sub_hierarchies(&self) -> Vec<Rc<dyn IDataProvider<Comment, Target>>> {
        Vec::new()
    }

    fn attach(&self, comment: &Comment, target: &Target) {
        self.attachments.borrow_mut().push((comment.id, target.id));
    }
}

struct MapExplicitProvider {
    attachments: HashMap<usize, Target>,
}

impl IExplicitAttachmentProvider<Comment, Target> for MapExplicitProvider {
    fn find_explicit_attachment(&self, comment: &Comment) -> Option<Target> {
        self.attachments.get(&comment.id).cloned()
    }
}

#[test]
fn comment_attacher_strict_reference_attaches_single_target() {
    let comments = vec![
        Comment {
            id: 1,
            text: "See NodeA".to_string(),
        },
        Comment {
            id: 2,
            text: "NodeA and NodeB".to_string(),
        },
    ];
    let targets = vec![
        Target {
            id: 10,
            name: "NodeA".to_string(),
        },
        Target {
            id: 20,
            name: "NodeB".to_string(),
        },
    ];

    let provider = SimpleDataProvider::new(comments, targets);

    let mut matcher = NodeReferenceMatcher::new();
    matcher
        .with_comment_text_provider(|c: &Comment| c.text.clone())
        .with_target_name_provider(|t: &Target| t.name.clone());

    let mut attacher = CommentAttacher::new();
    attacher.add_matcher(Rc::new(matcher));
    attacher.attach_comments(&provider);

    assert_eq!(provider.attachments(), vec![(1, 10)]);
}

#[test]
fn comment_attacher_fuzzy_reference_allows_whitespace_and_case() {
    let comments = vec![Comment {
        id: 1,
        text: "my   node".to_string(),
    }];
    let targets = vec![Target {
        id: 10,
        name: "MyNode".to_string(),
    }];

    let provider = SimpleDataProvider::new(comments, targets);

    let mut matcher = NodeReferenceMatcher::new();
    matcher
        .with_comment_text_provider(|c: &Comment| c.text.clone())
        .with_target_name_provider(|t: &Target| t.name.clone())
        .with_fuzzy_matching();

    let mut attacher = CommentAttacher::new();
    attacher.add_matcher(Rc::new(matcher));
    attacher.attach_comments(&provider);

    assert_eq!(provider.attachments(), vec![(1, 10)]);
}

#[test]
fn explicit_attachments_disable_heuristics_by_default() {
    let comments = vec![
        Comment {
            id: 1,
            text: "NodeA".to_string(),
        },
        Comment {
            id: 2,
            text: "NodeB".to_string(),
        },
    ];
    let targets = vec![
        Target {
            id: 10,
            name: "NodeA".to_string(),
        },
        Target {
            id: 20,
            name: "NodeB".to_string(),
        },
    ];

    let provider = SimpleDataProvider::new(comments, targets.clone());
    let explicit_provider = MapExplicitProvider {
        attachments: vec![(1, targets[0].clone())].into_iter().collect(),
    };

    let mut matcher = NodeReferenceMatcher::new();
    matcher
        .with_comment_text_provider(|c: &Comment| c.text.clone())
        .with_target_name_provider(|t: &Target| t.name.clone());

    let mut attacher = CommentAttacher::new();
    attacher.with_explicit_attachment_provider(Some(Rc::new(explicit_provider)));
    attacher.add_matcher(Rc::new(matcher));
    attacher.attach_comments(&provider);

    assert_eq!(provider.attachments(), vec![(1, 10)]);
}

#[test]
fn keep_heuristics_enabled_with_explicit_attachments() {
    let comments = vec![
        Comment {
            id: 1,
            text: "NodeA".to_string(),
        },
        Comment {
            id: 2,
            text: "NodeB".to_string(),
        },
    ];
    let targets = vec![
        Target {
            id: 10,
            name: "NodeA".to_string(),
        },
        Target {
            id: 20,
            name: "NodeB".to_string(),
        },
    ];

    let provider = SimpleDataProvider::new(comments, targets.clone());
    let explicit_provider = MapExplicitProvider {
        attachments: vec![(1, targets[0].clone())].into_iter().collect(),
    };

    let mut matcher = NodeReferenceMatcher::new();
    matcher
        .with_comment_text_provider(|c: &Comment| c.text.clone())
        .with_target_name_provider(|t: &Target| t.name.clone());

    let mut attacher = CommentAttacher::new();
    attacher
        .with_explicit_attachment_provider(Some(Rc::new(explicit_provider)))
        .keep_heuristics_enabled_with_explicit_attachments(true)
        .add_matcher(Rc::new(matcher));
    attacher.attach_comments(&provider);

    assert_eq!(provider.attachments(), vec![(1, 10), (2, 20)]);
}

#[test]
fn text_prefix_filter_rejects_prefix_matches() {
    let comments = vec![
        Comment {
            id: 1,
            text: "NOTE: NodeA".to_string(),
        },
        Comment {
            id: 2,
            text: "NodeA".to_string(),
        },
    ];
    let targets = vec![Target {
        id: 10,
        name: "NodeA".to_string(),
    }];

    let provider = SimpleDataProvider::new(comments, targets);

    let mut matcher = NodeReferenceMatcher::new();
    matcher
        .with_comment_text_provider(|c: &Comment| c.text.clone())
        .with_target_name_provider(|t: &Target| t.name.clone());

    let mut prefix_filter: TextPrefixFilter<Comment, Target> = TextPrefixFilter::new();
    prefix_filter
        .with_comment_text_provider(|c: &Comment| c.text.clone())
        .add_prefix("NOTE:");

    let mut attacher = CommentAttacher::new();
    attacher
        .add_filter(Rc::new(prefix_filter))
        .add_matcher(Rc::new(matcher));
    attacher.attach_comments(&provider);

    assert_eq!(provider.attachments(), vec![(2, 10)]);
}

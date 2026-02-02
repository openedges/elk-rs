use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::comments::{
    AlignmentMatcher, DistanceMatcher, IBoundsProvider, IDataProvider, IFilter, IMatcher,
    SizeFilter,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;

#[derive(Clone, Debug)]
struct CommentItem {
    bounds: ElkRectangle,
}

#[derive(Clone, Debug)]
struct TargetItem {
    bounds: ElkRectangle,
}

struct TestBoundsProvider;

impl IBoundsProvider<CommentItem, TargetItem> for TestBoundsProvider {
    fn bounds_for_comment(&self, comment: &CommentItem) -> Option<ElkRectangle> {
        Some(comment.bounds)
    }

    fn bounds_for_target(&self, target: &TargetItem) -> Option<ElkRectangle> {
        Some(target.bounds)
    }
}

struct DummyProvider;

impl IDataProvider<CommentItem, TargetItem> for DummyProvider {
    fn provide_comments(&self) -> Vec<CommentItem> {
        Vec::new()
    }

    fn provide_targets(&self) -> Vec<TargetItem> {
        Vec::new()
    }

    fn provide_sub_hierarchies(&self) -> Vec<Rc<dyn IDataProvider<CommentItem, TargetItem>>> {
        Vec::new()
    }

    fn attach(&self, _comment: &CommentItem, _target: &TargetItem) {}
}

#[test]
fn distance_matcher_intersection_normalizes_to_one() {
    let bounds_provider = Rc::new(TestBoundsProvider);
    let mut matcher = DistanceMatcher::new();
    matcher
        .with_bounds_provider(bounds_provider)
        .with_maximum_attachment_distance(5.0);

    let comment = CommentItem {
        bounds: ElkRectangle::with_values(0.0, 0.0, 10.0, 10.0),
    };
    let target = TargetItem {
        bounds: ElkRectangle::with_values(5.0, 5.0, 10.0, 10.0),
    };

    assert_eq!(matcher.normalized(&comment, &target), 1.0);
}

#[test]
fn distance_matcher_far_normalizes_to_zero() {
    let bounds_provider = Rc::new(TestBoundsProvider);
    let mut matcher = DistanceMatcher::new();
    matcher
        .with_bounds_provider(bounds_provider)
        .with_maximum_attachment_distance(5.0);

    let comment = CommentItem {
        bounds: ElkRectangle::with_values(0.0, 0.0, 10.0, 10.0),
    };
    let target = TargetItem {
        bounds: ElkRectangle::with_values(30.0, 0.0, 10.0, 10.0),
    };

    assert_eq!(matcher.normalized(&comment, &target), 0.0);
}

#[test]
fn alignment_matcher_intersection_normalizes_to_one() {
    let bounds_provider = Rc::new(TestBoundsProvider);
    let mut matcher = AlignmentMatcher::new();
    matcher
        .with_bounds_provider(bounds_provider)
        .with_maximum_alignment_offset(5.0);

    let comment = CommentItem {
        bounds: ElkRectangle::with_values(0.0, 0.0, 10.0, 10.0),
    };
    let target = TargetItem {
        bounds: ElkRectangle::with_values(0.0, 0.0, 10.0, 10.0),
    };

    assert_eq!(matcher.normalized(&comment, &target), 1.0);
}

#[test]
fn alignment_matcher_diagonal_normalizes_to_zero() {
    let bounds_provider = Rc::new(TestBoundsProvider);
    let mut matcher = AlignmentMatcher::new();
    matcher
        .with_bounds_provider(bounds_provider)
        .with_maximum_alignment_offset(5.0);

    let comment = CommentItem {
        bounds: ElkRectangle::with_values(0.0, 0.0, 10.0, 10.0),
    };
    let target = TargetItem {
        bounds: ElkRectangle::with_values(20.0, 20.0, 10.0, 10.0),
    };

    assert_eq!(matcher.normalized(&comment, &target), 0.0);
}

#[test]
fn size_filter_respects_max_area() {
    let bounds_provider = Rc::new(TestBoundsProvider);
    let mut filter = SizeFilter::new();
    filter
        .with_bounds_provider(bounds_provider)
        .with_maximum_area(50.0);

    filter.preprocess(&DummyProvider, false);

    let small = CommentItem {
        bounds: ElkRectangle::with_values(0.0, 0.0, 5.0, 5.0),
    };
    let large = CommentItem {
        bounds: ElkRectangle::with_values(0.0, 0.0, 10.0, 10.0),
    };

    assert!(filter.eligible_for_attachment(&small));
    assert!(!filter.eligible_for_attachment(&large));
}

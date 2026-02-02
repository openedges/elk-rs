use crate::org::eclipse::elk::core::service::LayoutMapping;
use crate::org::eclipse::elk::core::util::IElkProgressMonitor;

pub trait ILayoutListener {
    fn layout_about_to_start(&self, mapping: &LayoutMapping, progress_monitor: &mut dyn IElkProgressMonitor);
    fn layout_done(&self, mapping: &LayoutMapping, progress_monitor: &mut dyn IElkProgressMonitor);
}

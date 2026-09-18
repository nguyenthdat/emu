//! Interactive TUI research components, panels, and modal dialogs.
//!
//! Defined in accordance with plan.md §Structure, Principle II, Principle IV, FR-048, and SC-015.

pub mod app_panel;
pub mod companion_panel;
pub mod dialogs;
pub mod frida_panel;
pub mod guest_table;
pub mod image_panel;
pub mod kernel_panel;
pub mod profile_panel;
pub mod root_status;

pub use app_panel::AppPanelWidget;
pub use companion_panel::CompanionPanelWidget;
pub use dialogs::SafetyGateModal;
pub use frida_panel::FridaPanelWidget;
pub use guest_table::GuestTableWidget;
pub use image_panel::ImagePanelWidget;
pub use kernel_panel::KernelPanelWidget;
pub use profile_panel::ProfilePanelWidget;
pub use root_status::RootStatusWidget;

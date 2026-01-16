mod app;
mod config;
mod package_manager;
mod polkit;
mod virtualized_list;

use app::CosmicAppletPackageUpdater;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<CosmicAppletPackageUpdater>(())
}

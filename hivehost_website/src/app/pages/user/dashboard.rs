use leptos::prelude::ElementChild;
use leptos::{component, view, IntoView};

#[component]
pub fn DashboardPage() -> impl IntoView {
    view! {
        <div>
            <h2>Dashboard</h2>
        </div>
    }
}

pub mod server_fns {
    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    }}
}

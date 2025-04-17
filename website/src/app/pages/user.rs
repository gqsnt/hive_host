use leptos::{component, view, IntoView};
use leptos::prelude::{expect_context};
use leptos_router::components::{Outlet, A};
use crate::app::ReadUserSignal;
use leptos::prelude::ElementChild;

pub mod projects;
pub mod user_info;

#[component]
pub  fn UserPage() -> impl IntoView{
    let _user = expect_context::<ReadUserSignal>();
    view! {
        <h2>"User Parent"</h2>
        <div>
            <A href="/user/projects">"Projects"</A>
            <A href="/user">"User Info"</A>
        </div>
        <Outlet />
    }
}






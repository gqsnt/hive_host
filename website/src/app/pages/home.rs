use leptos::{component, view, IntoView};
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;

#[component]
pub fn HomePage() -> impl IntoView {
    view! { <h1 class="text-red-500 ">"Welcome to Hive Host!"</h1> }
}


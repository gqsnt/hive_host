use leptos::prelude::{AddAnyAttr, ServerAction};
use leptos::{component, view, IntoView};
use leptos::either::Either;
use leptos::prelude::{expect_context, ClassAttribute, Read};
use leptos::prelude::ElementChild;
use leptos::prelude::IntoAnyAttribute;

use leptos_router::components::A;
use crate::app::ReadUserSignal;
use crate::security::logout;
use leptos::prelude::IntoMaybeErased;


#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <nav class="flex items-center justify-between p-6 lg:px-8">
            <A attr:class="-m-1.5 p-1.5" href="/">
                <span class="sr-only">Hive Host</span>
                <img
                    class="h-8 w-auto"
                    src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600"
                    alt=""
                />
            </A>
            <A href="/login" attr:class="text-sm/6 font-semibold text-gray-900 ml-4">
                Log in ->
            </A>
        </nav>
        <h1>"Welcome to Hive Host!"</h1>
    }
}


use leptos::ev::Targeted;
use leptos::prelude::*;
use web_sys::{Event, HtmlSelectElement};

#[component]
pub fn FormSelect(
    name: String,
    #[prop(optional)] value: Option<String>,
    children: Children,
    #[prop(optional, into)] on_change: Option<Callback<String>>,
) -> impl IntoView {
    let on_target_input = move |ev:Targeted<Event, HtmlSelectElement>|{
       if let Some(on_change) = on_change .as_ref(){
            on_change.try_run(ev.target().value());
        }
    } ;
    view! {
        <div class="relative">
            <select name=name class="form-select" prop:value=value on:input:target=on_target_input>

                {children()}
            </select>
            <svg
                class="select-icon"
                viewBox="0 0 16 16"
                fill="currentColor"
                aria-hidden="true"
                data-slot="icon"
            >
                <path d="M4.22 6.22a.75.75 0 0 1 1.06 0L8 8.94l2.72-2.72" />
            </svg>
        </div>
    }
}

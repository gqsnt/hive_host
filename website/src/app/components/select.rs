use leptos::prelude::*;


#[component]
pub fn FormSelectIcon() -> impl IntoView{
    view! {
        <svg
            class="select-icon"
            viewBox="0 0 16 16"
            fill="currentColor"
            aria-hidden="true"
            data-slot="icon"
        >
            <path d="M4.22 6.22a.75.75 0 0 1 1.06 0L8 8.94l2.72-2.72" />
        </svg>
    }
}




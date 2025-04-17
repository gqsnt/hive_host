use leptos::{component, view, IntoView};
use leptos::prelude::{ActionForm, ClassAttribute, ServerAction};
use leptos::prelude::ElementChild;
use crate::security::Logout;

#[component]
pub fn LogoutButton(action: ServerAction<Logout>) -> impl IntoView {
    view! {
        <div class="hidden lg:flex lg:flex-1 lg:justify-end">
            <ActionForm action=action>
                <button class="text-sm/6 font-semibold text-gray-900" type="submit">
                    "Log Out"
                </button>
            </ActionForm>
        </div>
    }
}
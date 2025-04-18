
use leptos::prelude::{AddAnyAttr, IntoAny};
use leptos::{component, server, view, IntoView};
use leptos::attr::Formaction;
use leptos::control_flow::For;
use leptos::either::Either;
use leptos::logging::log;
use leptos::prelude::{expect_context, ActionForm, ClassAttribute, ElementExt, FromFormData, Get, OnAttribute, OnceResource, Read, Resource, ServerAction, ServerFnError, Show, Signal, Suspense};
use crate::app::ReadUserSignal;
use leptos::prelude::ElementChild;
use leptos::text_prop::TextProp;
use web_sys::SubmitEvent;
use crate::models::SshKeyInfo;
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::IntoAnyAttribute;
use common::server_action::user_action::UserAction;


#[component]
pub  fn UserSettingsPage() -> impl IntoView{
    let _user = expect_context::<ReadUserSignal>();
    let delete_ssh_action = ServerAction::<DeleteSshKey>::new();
    let add_ssh_action = ServerAction::<AddSshKey>::new();
    let update_password_action = ServerAction::<UpdatePassword>::new();
    let ssh_keys_resource = Resource::new(
        move || (delete_ssh_action.version().get() ,add_ssh_action.version().get()),
        |_| get_ssh_keys()
    );
    let csrf = OnceResource::new(crate::app::components::csrf_field::generate_csrf());
    let get_csrf = move ||{
        csrf.get().map(|c| c.clone().unwrap_or_default()).unwrap_or_default()
    };

    let input_class = "block w-full rounded-md bg-white/5 px-3 py-1.5 text-base text-white outline-1 -outline-offset-1 outline-white/10 placeholder:text-gray-500 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-500 sm:text-sm/6 cursor-pointer";
    let label_class = "block text-sm/6 font-medium text-white";
    let section_title_class = "text-base/7 font-semibold text-white mt-2";
    let section_desc_class = "mt-1 text-sm/6 text-gray-400";
    let button_primary_class = "rounded-md bg-indigo-500 px-3 py-2 text-sm font-semibold text-white shadow-xs hover:bg-indigo-400 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500 disabled:opacity-50";
    let button_danger_class = "rounded-md bg-red-500 px-3 py-2 text-sm font-semibold text-white shadow-xs hover:bg-red-400 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-500 disabled:opacity-50 cursor-pointer";


    view! {
        // --- Profile Section (Empty as requested) ---
        <div class="border-b border-white/10 pb-12">
            <h2 class=section_title_class>"Profile"</h2>
            <p class=section_desc_class>
                "This section is currently empty. Future profile settings will appear here."
            </p>
            // </div>
            <div class="mt-10"></div>

            // --- Security Section (Password Change) ---
            <div class="border-b border-white/10 pb-12">
                <h2 class=section_title_class>"Security"</h2>
                <p class=section_desc_class>"Update your account password."</p>

                // Separate form for password change for clarity
                <ActionForm action=update_password_action>
                    <input type="hidden" value=get_csrf name="csrf" />
                    <div class="mt-10 grid grid-cols-1 gap-x-6 gap-y-8 sm:grid-cols-6">
                        <div class="sm:col-span-4">
                            <label for="old_password" class=label_class>
                                "Old Password"
                            </label>
                            <div class="mt-2">
                                <input
                                    type="password"
                                    name="old_password"
                                    required
                                    class=input_class
                                />
                            </div>
                        </div>

                        <div class="sm:col-span-4">
                            <label for="new_password" class=label_class>
                                "New Password"
                            </label>
                            <div class="mt-2">
                                <input
                                    type="password"
                                    name="new_password"
                                    required
                                    class=input_class
                                />
                            </div>
                        </div>

                        <div class="sm:col-span-4">
                            <label for="confirm_password" class=label_class>
                                "Confirm New Password"
                            </label>
                            <div class="mt-2">
                                <input
                                    type="password"
                                    name="confirm_password"
                                    required
                                    class=input_class
                                />
                            </div>
                        </div>
                    </div>

                    // --- Action Feedback and Submit Button ---
                    <div class="mt-6 flex items-center justify-end gap-x-6">
                        // Feedback area
                        <div class="text-sm mr-auto">
                            <Show when=move || update_password_action.pending().get()>
                                <p class="text-gray-400">"Updating password..."</p>
                            </Show>
                            {move || {
                                update_password_action
                                    .value()
                                    .get()
                                    .map(|result| {
                                        match result {
                                            Ok(_) => {
                                                Either::Left(
                                                    view! {
                                                        <p class="text-green-400">
                                                            "Password updated successfully!"
                                                        </p>
                                                    },
                                                )
                                            }
                                            Err(e) => {
                                                Either::Right(
                                                    view! {
                                                        <p class="text-red-400">{format!("Error: {}", e)}</p>
                                                    },
                                                )
                                            }
                                        }
                                    })
                            }}
                        </div>
                        <button
                            type="submit"
                            disabled=move || update_password_action.pending().get()
                            class=button_primary_class
                        >
                            "Change Password"
                        </button>
                    </div>
                </ActionForm>
            </div>
            <div class="border-b border-white/10 pb-12">
                <h2 class=section_title_class>"SSH Keys"</h2>
                <p class=section_desc_class>
                    "Manage SSH keys used for accessing your account via Git."
                </p>

                // --- Flex container for List and Form ---
                // Stacks vertically on small screens, row on medium+
                // Adjust gap and padding as needed
                <div class="mt-8 flex flex-col md:flex-row md:gap-x-8">

                    // --- Left Side: List of Existing Keys ---
                    // Use flex-1 to allow it to grow, or set a specific width like md:w-3/5
                    <div class="flex-1 mb-8 md:mb-0">
                        <h3 class="text-base font-medium text-white mb-4">"Existing Keys"</h3>
                        // Keep flow-root for potential overflow handling within the table container
                        <div class="flow-root">
                            <div class="-mx-4 -my-2 overflow-x-auto sm:-mx-6 lg:-mx-8">
                                <div class="inline-block min-w-full py-2 align-middle sm:px-6 lg:px-8">
                                    <table class="min-w-full divide-y divide-gray-700">
                                        // --- Refactored thead ---
                                        <thead>
                                            <tr>
                                                // Single header spanning the content area
                                                <th
                                                    scope="col"
                                                    class="py-3.5 pr-3 pl-4 text-left text-sm font-semibold text-white sm:pl-0"
                                                >
                                                    "Key Name & Actions"
                                                </th>
                                            </tr>
                                        </thead>
                                        <tbody class="divide-y divide-gray-800">
                                            <Suspense fallback=move || {
                                                view! { <SingleColLoadingRow /> }
                                            }>
                                                {move || {
                                                    ssh_keys_resource
                                                        .get()
                                                        .map(|keys_result| match keys_result {
                                                            Ok(keys) => {
                                                                Either::Left({
                                                                    if keys.is_empty() {
                                                                        Either::Left(
                                                                            view! {
                                                                                <SingleColMessageRow message="No SSH keys added yet." />
                                                                            },
                                                                        )
                                                                    } else {
                                                                        Either::Right(
                                                                            view! {
                                                                                <For
                                                                                    each=move || keys.clone()
                                                                                    key=|key| key.id
                                                                                    // Using children prop syntax
                                                                                    children=move |key| {
                                                                                        let ssh_key_id = key.id;
                                                                                        let key_name_to_delete = key.name.clone();
                                                                                        let on_delete_click = move |ev: SubmitEvent| {
                                                                                            let confirmed = if let Some(window) = web_sys::window() {
                                                                                                window
                                                                                                    .confirm_with_message(
                                                                                                        &format!(
                                                                                                            "Are you sure you want to delete the key '{}'?",
                                                                                                            key_name_to_delete,
                                                                                                        ),
                                                                                                    )
                                                                                                    .unwrap_or(false)
                                                                                            } else {
                                                                                                false
                                                                                            };
                                                                                            if !confirmed {
                                                                                                ev.prevent_default();
                                                                                            }
                                                                                        };

                                                                                        view! {
                                                                                            <tr>
                                                                                                // --- Refactored td using flex ---
                                                                                                <td class="py-4 pr-3 pl-4 text-sm font-medium whitespace-nowrap text-white sm:pl-0">
                                                                                                    // Use justify-between to push items apart
                                                                                                    <div class="flex items-center justify-between gap-x-4">
                                                                                                        // Key Name on the left
                                                                                                        // Use truncate if names can be long
                                                                                                        <span class="truncate">{key.name}</span>

                                                                                                        // Delete Button on the right
                                                                                                        <ActionForm
                                                                                                            action=delete_ssh_action
                                                                                                            on:submit=on_delete_click
                                                                                                        >
                                                                                                            <input type="hidden" value=get_csrf name="csrf" />
                                                                                                            <input type="hidden" name="ssh_key_id" value=key.id />
                                                                                                            <button
                                                                                                                type="submit"
                                                                                                                class=button_danger_class
                                                                                                                disabled=move || delete_ssh_action.pending().get()
                                                                                                            >
                                                                                                                "Delete"
                                                                                                            </button>
                                                                                                        </ActionForm>

                                                                                                    </div>
                                                                                                </td>
                                                                                            </tr>
                                                                                        }
                                                                                    }
                                                                                />
                                                                            },
                                                                        )
                                                                    }
                                                                })
                                                            }
                                                            Err(e) => {
                                                                Either::Right(
                                                                    // End children closure
                                                                    // End For component
                                                                    // End Either::Right view!
                                                                    // End else
                                                                    // End Either::Left Ok case
                                                                    view! {
                                                                        <SingleColMessageRow
                                                                            message=format!("Error loading SSH keys: {}", e)
                                                                            is_error=true
                                                                        />
                                                                    },
                                                                )
                                                            }
                                                        })
                                                }}
                                            </Suspense>
                                        </tbody>
                                    </table>
                                    // Feedback for delete action
                                    // Use .output()
                                    {move || {
                                        delete_ssh_action
                                            .value()
                                            .get()
                                            .map(|result| {
                                                match result {
                                                    Ok(_) => Either::Left(view! {}),
                                                    Err(e) => {
                                                        Either::Right(
                                                            // Feedback might be less useful here if the list just updates.
                                                            // Consider removing or making it appear briefly.
                                                            view! {
                                                                <p class="mt-2 text-sm text-red-400">
                                                                    {format!("Error deleting key: {}", e)}
                                                                </p>
                                                            },
                                                        )
                                                    }
                                                }
                                            })
                                    }}
                                </div>
                            </div>
                        </div>
                    // End Left Side
                    </div>

                    // --- Separator ---
                    // Visible on medium screens and up
                    <div class="hidden md:block border-l border-white/10"></div>

                    // --- Right Side: Form to Add New Key ---
                    // Use flex-1 or set a specific width like md:w-2/5
                    // Add padding left on medium+ to visually separate from border
                    <div class="flex-1 md:pl-2">
                        <h3 class="text-base font-medium text-white mb-4">"Add New SSH Key"</h3>
                        <ActionForm action=add_ssh_action>
                            <input type="hidden" value=get_csrf name="csrf" />
                            // Keep grid for form layout
                            <div class="grid grid-cols-1 gap-x-6 gap-y-6 sm:grid-cols-6">
                                // Adjust span if needed based on parent width
                                <div class="sm:col-span-6">
                                    <label for="ssh_key_name" class=label_class>
                                        "Key Name / Label"
                                    </label>
                                    <div class="mt-2">
                                        <input
                                            type="text"
                                            name="ssh_key_name"
                                            required
                                            placeholder="e.g., My Work Laptop"
                                            class=input_class
                                        />
                                    </div>
                                </div>

                                <div class="col-span-full">
                                    <label for="ssh_key_value" class=label_class>
                                        "SSH Key"
                                    </label>
                                    <div class="mt-2">
                                        <textarea
                                            name="ssh_key_value"
                                            // Adjust rows as needed for space
                                            rows="5"
                                            required
                                            placeholder="Begins with ssh-rsa, ssh-ed25519, etc."
                                            class="block w-full rounded-md bg-white/5 px-3 py-1.5 text-base text-white outline-1 -outline-offset-1 outline-white/10 placeholder:text-gray-500 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-500 sm:text-sm/6"
                                        ></textarea>
                                    </div>
                                </div>
                            </div>

                            // --- Action Feedback and Submit Button ---
                            <div class="mt-6 flex items-center justify-end gap-x-6">
                                // Feedback area
                                <div class="text-sm mr-auto">
                                    <Show when=move || add_ssh_action.pending().get()>
                                        <p class="text-gray-400">"Adding key..."</p>
                                    </Show>
                                    {move || {
                                        add_ssh_action
                                            .value()
                                            .get()
                                            .map(|result| {
                                                match result {
                                                    Ok(_) => {
                                                        Either::Left(
                                                            view! {
                                                                <p class="text-green-400">"SSH Key added successfully!"</p>
                                                            },
                                                        )
                                                    }
                                                    Err(e) => {
                                                        Either::Right(
                                                            view! {
                                                                <p class="text-red-400">{format!("Error: {}", e)}</p>
                                                            },
                                                        )
                                                    }
                                                }
                                            })
                                    }}
                                </div>
                                <button
                                    type="submit"
                                    disabled=move || add_ssh_action.pending().get()
                                    class=button_primary_class
                                >
                                    "Add SSH Key"
                                </button>
                            </div>
                        </ActionForm>
                    // End Right Side
                    </div>

                // End Flex Container
                </div>
            // End SSH Keys Section
            </div>
        </div>
    }
}


#[component]
fn LoadingRow() -> impl IntoView {
    view! {
        <tr>
            <td
                colspan="2"
                class="py-4 pr-3 pl-4 text-sm text-center whitespace-nowrap text-gray-400 sm:pl-0"
            >
                "Loading keys..."
            </td>
        </tr>
    }
}

#[component]
fn EmptyRow(
    #[prop(into)]
    message: TextProp
) -> impl IntoView {
    view! {
        <tr>
            <td
                colspan="2"
                class="py-4 pr-3 pl-4 text-sm text-center whitespace-nowrap text-gray-400 sm:pl-0"
            >
                {message.get()}
            </td>
        </tr>
    }
}

#[component]
fn ErrorRow(
    #[prop(into)]
    message: TextProp
) -> impl IntoView {
    view! {
        <tr>
            <td
                colspan="2"
                class="py-4 pr-3 pl-4 text-sm text-center whitespace-nowrap text-red-400 sm:pl-0"
            >
                {message.get()}
            </td>
        </tr>
    }
}


#[component]
fn SingleColLoadingRow() -> impl IntoView {
    view! {
        <tr>
            <td class="py-4 pr-3 pl-4 text-sm text-center whitespace-nowrap text-gray-400 sm:pl-0">
                "Loading keys..."
            </td>
        </tr>
    }
}

#[component]
fn SingleColMessageRow(
    #[prop(into)] message: TextProp,
    #[prop(optional, into)] is_error: Signal<bool>
) -> impl IntoView {
    let text_color_class = move || if is_error.get() { "text-red-400" } else { "text-gray-400" };
    view! {
        <tr>
            <td class=move || {
                format!(
                    "py-4 pr-3 pl-4 text-sm text-center whitespace-nowrap sm:pl-0 {}",
                    text_color_class(),
                )
            }>{message.get()}</td>
        </tr>
    }
}



#[server]
pub async fn get_ssh_keys() -> Result<Vec<SshKeyInfo>, ServerFnError> {
    use crate::ssr::auth;
    use crate::ssr::server_vars;
    use crate::ssr::pool;
    use crate::models::User;
    use crate::security::utils::ssr::get_auth_session_user_id;

    let auth = auth(false)?;
    let pool = pool()?;
    let user_id = get_auth_session_user_id(&auth).unwrap();
    Ok(sqlx::query_as!(
        SshKeyInfo,
        r#"
        SELECT id, name, user_id FROM user_ssh_keys WHERE user_id = $1
        "#,
        user_id
    )
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?)
}


#[server]
pub async fn delete_ssh_key(csrf:String, ssh_key_id:i64) -> Result<(), ServerFnError> {
    use crate::ssr::auth;
    use crate::ssr::server_vars;
    use crate::ssr::pool;
    use crate::models::User;
    use crate::security::utils::ssr::get_auth_session_user_id;
    use crate::security::utils::ssr::verify_easy_hash;
    use crate::api::ssr::request_server_action;

    let auth = auth(false)?;
    let pool = pool()?;
    let server_vars = server_vars()?;
    verify_easy_hash(
        auth.session.get_session_id().to_string(),
        server_vars.csrf_server.to_secret(),
        csrf,
    )?;
    let user_id = get_auth_session_user_id(&auth).unwrap();
    let record = sqlx::query!(
        r#"
        DELETE FROM user_ssh_keys WHERE id = $1 AND user_id = $2 returning public_key
        "#,
        ssh_key_id,
        user_id
    )
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let _ = request_server_action(UserAction::RemoveSshKey {
        user_slug: auth.current_user.unwrap_or_default().get_slug(),
        ssh_key: record.public_key,
    }.into()).await;
    Ok(())
}


#[server]
pub async fn add_ssh_key(csrf: String,ssh_key_name: String,ssh_key_value: String) -> Result<(), ServerFnError> {
    use crate::ssr::auth;
    use crate::ssr::server_vars;
    use crate::ssr::pool;
    use crate::models::User;
    use crate::security::utils::ssr::get_auth_session_user_id;
    use crate::security::utils::ssr::verify_easy_hash;
    use crate::api::ssr::request_server_action;

    let auth = auth(false)?;
    let pool = pool()?;
    let server_vars = server_vars()?;
    verify_easy_hash(
        auth.session.get_session_id().to_string(),
        server_vars.csrf_server.to_secret(),
        csrf,
    )?;
    let user_id = get_auth_session_user_id(&auth).unwrap();
    let ssh_key_record = sqlx::query!(
        r#"
        INSERT INTO user_ssh_keys (name, public_key, user_id) VALUES ($1, $2, $3)
        "#,
        ssh_key_name,
        ssh_key_value,
        user_id
    )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let _ = request_server_action(UserAction::AddSshKey {
        user_slug: auth.current_user.unwrap_or_default().get_slug(),
        ssh_key: ssh_key_value,
    }.into()).await;
    Ok(())
}



#[server]
pub async fn update_password(
    csrf: String,
    old_password: String,
    new_password: String,
    new_password_confirmation: String,
) -> Result<(), ServerFnError> {
    use crate::ssr::auth;
    use crate::ssr::server_vars;
    use crate::ssr::pool;
    use crate::models::User;
    use crate::security::utils::ssr::get_auth_session_user_id;
    use crate::security::utils::ssr::verify_easy_hash;


    let auth = auth(false)?;
    let server_vars = server_vars()?;
    verify_easy_hash(
        auth.session.get_session_id().to_string(),
        server_vars.csrf_server.to_secret(),
        csrf,
    )?;

    Ok(())
}
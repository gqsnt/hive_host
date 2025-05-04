use leptos::control_flow::For;
use leptos::either::Either;

use leptos::prelude::{expect_context, ElementChild, NodeRef, NodeRefAttribute, OnAttribute};
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::{signal, Effect, Set};
use leptos::prelude::{ClassAttribute, Get, Resource, ServerAction, Signal, Suspense};
use leptos::text_prop::TextProp;
use leptos::{component, view, IntoView};
use leptos::html::{Input, Textarea};
use reactive_stores::Store;
use web_sys::SubmitEvent;
use crate::app::pages::{GlobalState, GlobalStateStoreFields};

#[component]
pub fn UserSettingsPage() -> impl IntoView {
    let global_state: Store<GlobalState> = expect_context();
    let delete_ssh_action = ServerAction::<server_fns::DeleteSshKey>::new();
    let add_ssh_action = ServerAction::<server_fns::AddSshKey>::new();
    let update_password_action = ServerAction::<server_fns::UpdatePassword>::new();

    let ssh_keys_resource = Resource::new_bitcode(
        move || {
            (
                delete_ssh_action.version().get(),
                add_ssh_action.version().get(),
            )
        },
        |_| server_fns::get_ssh_keys(),
    );


    let old_password_ref = NodeRef::<Input>::default();
    let new_password_ref = NodeRef::<Input>::default();
    let new_password_confirm_ref = NodeRef::<Input>::default();

    let add_ssh_key_name_ref = NodeRef::<Input>::default();
    let add_ssh_key_value_ref = NodeRef::<Textarea>::default();


    let on_password_update = move |event: SubmitEvent| {
        event.prevent_default();
        update_password_action.dispatch(server_fns::UpdatePassword {
            csrf: global_state.csrf().get().unwrap_or_default(),
            old_password: old_password_ref
                .get()
                .expect("<input> should be mounted")
                .value(),
            new_password: new_password_ref
                .get()
                .expect("<input> should be mounted")
                .value(),
            new_password_confirm: new_password_confirm_ref
                .get()
                .expect("<input> should be mounted")
                .value(),
        });
    };



    let on_delete_ssh_click = move |event: SubmitEvent, ssh_key_id:i64, key_name:String| {
        event.prevent_default();
        let confirmed = if let Some(window) = web_sys::window() {
            window
                .confirm_with_message(
                    &format!(
                        "Are you sure you want to delete the key '{key_name}'?",
                    ),
                )
                .unwrap_or(false)
        } else {
            false
        };
        if confirmed {
            delete_ssh_action.dispatch(server_fns::DeleteSshKey {
                csrf: global_state.csrf().get().unwrap_or_default(),
                ssh_key_id,
            });
        }

    };

    let add_ssh_key = move |event: SubmitEvent| {
        event.prevent_default();
        add_ssh_action.dispatch(server_fns::AddSshKey {
            csrf: global_state.csrf().get().unwrap_or_default(),
            ssh_key_name: add_ssh_key_name_ref
                .get()
                .expect("<input> should be mounted")
                .value(),
            ssh_key_value: add_ssh_key_value_ref
                .get()
                .expect("<input> should be mounted")
                .value(),
        });
    };



    let (new_ssh_key_result, set_new_ssh_key_result) = signal(" ".to_string());
    let (password_change_result, set_password_change_result) = signal(" ".to_string());

    Effect::new(move |_| {
        update_password_action.version().get();
        match update_password_action.value().get() {
            Some(Ok(_)) => set_password_change_result.set(String::from("Password changed")),
            Some(Err(e)) => set_password_change_result.set(e.to_string()),
            _ => (),
        };
    });

    Effect::new(move |_| {
        add_ssh_action.version().get();
        match add_ssh_action.value().get() {
            Some(Ok(_)) => set_new_ssh_key_result.set(String::from("SSh key added")),
            Some(Err(e)) => set_new_ssh_key_result.set(e.to_string()),
            _ => (),
        };
    });

    view! {
        // --- Profile Section (Empty as requested) ---
        <div class="h-full">
            <h2 class="section-title">"Profile"</h2>
            <p class="section-desc">
                "This section is currently empty. Future profile settings will appear here."
            </p>
            // </div>
            <div class="mt-10"></div>

            // --- Security Section (Password Change) ---
            <div class="section-border">
                <h2 class="section-title">"Security"</h2>
                <p class="section-desc">"Update your account password."</p>

                // Separate form for password change for clarity
                <form on:submit=on_password_update>
                    <div class="mt-10 grid grid-cols-1 gap-x-6 gap-y-8 sm:grid-cols-6">
                        <div class="sm:col-span-4">
                            <label for="old_password" class="form-label">
                                "Old Password"
                            </label>
                            <div class="mt-2">
                                <input
                                    type="password"
                                    name="old_password"
                                    node_ref=old_password_ref
                                    required
                                    class="form-input"
                                />
                            </div>
                        </div>

                        <div class="sm:col-span-4">
                            <label for="new_password" class="form-label">
                                "New Password"
                            </label>
                            <div class="mt-2">
                                <input
                                    type="password"
                                    node_ref=new_password_ref
                                    name="new_password"
                                    required
                                    class="form-input"
                                />
                            </div>
                        </div>

                        <div class="sm:col-span-4">
                            <label for="new_password_confirm" class="form-label">
                                "Confirm New Password"
                            </label>
                            <div class="mt-2">
                                <input
                                    type="password"
                                    name="new_password_confirm"
                                    node_ref=new_password_confirm_ref
                                    required
                                    class="form-input"
                                />
                            </div>
                        </div>
                    </div>

                    // --- Action Feedback and Submit Button ---
                    <div class="mt-6 flex items-center justify-end gap-x-6">
                        // Feedback area
                        <div class="text-sm mr-auto">{password_change_result}</div>
                        <button
                            type="submit"
                            disabled=move || update_password_action.pending().get()
                            class="btn btn-primary"
                        >
                            "Change Password"
                        </button>
                    </div>
                </form>

            </div>
            <div class="section-border">
                <h2 class="section-title">"SSH Keys"</h2>
                <p class="section-desc">
                    "Manage SSH keys used for accessing your account SFTP via SSH"
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
                                                                                        let key_name_to_delete = key.name.clone();

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
                                                                                                        <form on:submit=move |e| on_delete_ssh_click(
                                                                                                            e,
                                                                                                            key.id,
                                                                                                            key_name_to_delete.clone(),
                                                                                                        )>
                                                                                                            <button
                                                                                                                type="submit"
                                                                                                                class="btn btn-danger"
                                                                                                                disabled=move || delete_ssh_action.pending().get()
                                                                                                            >
                                                                                                                "Delete"
                                                                                                            </button>
                                                                                                        </form>

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
                        <form on:submit=add_ssh_key>
                            // Keep grid for form layout
                            <div class="grid grid-cols-1 gap-x-6 gap-y-6 sm:grid-cols-6">
                                // Adjust span if needed based on parent width
                                <div class="sm:col-span-6">
                                    <label for="ssh_key_name" class="form-label">
                                        "Key Name / Label"
                                    </label>
                                    <div class="mt-2">
                                        <input
                                            type="text"
                                            name="ssh_key_name"
                                            node_ref=add_ssh_key_name_ref
                                            required
                                            placeholder="e.g., My Work Laptop"
                                            class="form-input"
                                        />
                                    </div>
                                </div>

                                <div class="col-span-full">
                                    <label for="ssh_key_value" class="form-label">
                                        "SSH Key"
                                    </label>
                                    <div class="mt-2">
                                        <textarea
                                            name="ssh_key_value"
                                            // Adjust rows as needed for space
                                            rows="5"
                                            required
                                            node_ref=add_ssh_key_value_ref
                                            placeholder="Begins with ssh-rsa, ssh-ed25519, etc."
                                            class="block w-full rounded-md bg-white/5 px-3 py-1.5 text-base text-white outline-1 -outline-offset-1 outline-white/10 placeholder:text-gray-500 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-500 sm:text-sm/6"
                                        ></textarea>
                                    </div>
                                </div>
                            </div>

                            // --- Action Feedback and Submit Button ---
                            <div class="mt-6 flex items-center justify-end gap-x-6">
                                <button
                                    type="submit"
                                    disabled=move || add_ssh_action.pending().get()
                                    class="btn btn-primary"
                                >
                                    "Add SSH Key"
                                </button>
                            </div>
                            <div>{new_ssh_key_result}</div>

                        </form>
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
    #[prop(optional, into)] is_error: Signal<bool>,
) -> impl IntoView {
    let text_color_class = move || {
        if is_error.get() {
            "text-red-400"
        } else {
            "text-gray-400"
        }
    };
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

pub mod server_fns {
    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
        use crate::security::utils::ssr::get_auth_session_user_id;
        use crate::security::utils::ssr::verify_easy_hash;
        use crate::ssr::auth;
        use crate::ssr::pool;
        use crate::ssr::server_vars;
        use validator::Validate;
        use secrecy::ExposeSecret;
            use crate::security::utils::ssr::PasswordForm;
            use crate::app::pages::user::user_settings::server_fns::ssr::AddSshKeyForm;

    }}

    use crate::models::SshKeyInfo;
    use crate::AppResult;
    use leptos::server;
    use leptos::server_fn::codec::Bitcode;

    #[cfg(feature = "ssr")]
    mod ssr {
        use serde::{Deserialize, Serialize};
        use validator::Validate;
        #[derive(Validate, Debug, Serialize, Deserialize)]
        pub struct AddSshKeyForm {
            #[validate(length(min = 1, max = 20))]
            pub ssh_key_name: String,
            #[validate(length(min = 1, message = "SSH key value is required"))]
            pub ssh_key_value: String,
        }

        #[derive(Validate, Debug, Serialize, Deserialize)]
        pub struct GitSshKeyForm {
            #[validate(length(min = 1, message = "Git SSH key value cannot be empty"))]
            pub git_ssh_key_value: String,
        }
    }

    #[server(input=Bitcode, output=Bitcode)]
    pub async fn get_ssh_keys() -> AppResult<Vec<SshKeyInfo>> {
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
        .await?)
    }

    #[server(input=Bitcode, output=Bitcode)]
    pub async fn delete_ssh_key(csrf: String, ssh_key_id: i64) -> AppResult<()> {
        let auth = auth(false)?;
        let pool = pool()?;
        let server_vars = server_vars()?;
        verify_easy_hash(
            auth.session.get_session_id().to_string(),
            server_vars.csrf_server.to_secret(),
            csrf,
        )?;
        let user_id = get_auth_session_user_id(&auth).unwrap();
        let _ = sqlx::query!(
            r#"
        DELETE FROM user_ssh_keys WHERE id = $1 AND user_id = $2 returning public_key
        "#,
            ssh_key_id,
            user_id
        )
        .fetch_one(&pool)
        .await?;
        Ok(())
    }

    #[server]
    pub async fn add_ssh_key(
        csrf: String,
        ssh_key_name: String,
        ssh_key_value: String,
    ) -> AppResult<()> {
        let ssh_key_form = AddSshKeyForm {
            ssh_key_name: ssh_key_name.clone(),
            ssh_key_value: ssh_key_value.trim().to_string(),
        };
        ssh_key_form.validate()?;
        let auth = auth(false)?;
        let pool = pool()?;
        let server_vars = server_vars()?;
        verify_easy_hash(
            auth.session.get_session_id().to_string(),
            server_vars.csrf_server.to_secret(),
            csrf,
        )?;
        let user_id = get_auth_session_user_id(&auth).unwrap();
        let _ = sqlx::query!(
            r#"
        INSERT INTO user_ssh_keys (name, public_key, user_id) VALUES ($1, $2, $3)
        "#,
            ssh_key_name,
            ssh_key_value,
            user_id
        )
        .execute(&pool)
        .await?;
        Ok(())
    }

    #[server(input=Bitcode, output=Bitcode)]
    pub async fn update_password(
        csrf: String,
        old_password: String,
        new_password: String,
        new_password_confirm: String,
    ) -> AppResult<()> {
        let auth = auth(false)?;
        let server_vars = server_vars()?;
        verify_easy_hash(
            auth.session.get_session_id().to_string(),
            server_vars.csrf_server.to_secret(),
            csrf,
        )?;

        let password_form = PasswordForm {
            password: new_password.clone(),
            password_confirmation: new_password_confirm.clone(),
        };
        password_form.validate()?;
        let pool = pool()?;
        let user_id = get_auth_session_user_id(&auth).unwrap();

        // check old pwd
        let result = sqlx::query!(
            r#"
        SELECT password FROM users WHERE id = $1
        "#,
            user_id
        )
        .fetch_one(&pool)
        .await?;
        let password = secrecy::SecretString::from(old_password.as_str());
        password_auth::verify_password(password.expose_secret().as_bytes(), &result.password)
            .map_err(|_| crate::AppError::InvalidCredentials)?;

        // update password
        sqlx::query!(
            r#"
        UPDATE users SET password = $1 WHERE id = $2
        "#,
            password_auth::generate_hash(&new_password.as_bytes()),
            user_id
        )
        .execute(&pool)
        .await?;

        Ok(())
    }
}

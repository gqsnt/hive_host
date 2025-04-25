use crate::app::components::csrf_field::CSRFField;
use crate::app::pages::include_csrf;
use crate::security::signup::Signup;
use leptos::prelude::AddAnyAttr;
use leptos::prelude::IntoAnyAttribute;
use leptos::prelude::IntoMaybeErased;
use leptos::prelude::{signal, Effect, ElementChild, Get, ServerFnError, Set};
use leptos::prelude::{ActionForm, ClassAttribute, ServerAction};
use leptos::{component, view, IntoView};
use leptos_router::components::A;

#[component]
pub fn SignupPage() -> impl IntoView {
    include_csrf();

    let action = ServerAction::<Signup>::new();
    let (signup_result, set_signup_result) = signal(" ".to_string());
    Effect::new(move |_| {
        action.version().get();
        match action.value().get() {
            Some(Ok(_)) => set_signup_result.set(String::from("Signup Successful")),
            Some(Err(ServerFnError::ServerError(e))) => {
                set_signup_result.set(e)
            },
            _ => (),
        };
    });

    view! {
        <div class="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
            <div class="sm:mx-auto sm:w-full sm:max-w-sm">
                <img
                    class="mx-auto h-10 w-auto"
                    src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600"
                    alt="Hive Host"
                />
                <h2 class="mt-10 text-center text-2xl/9 font-bold tracking-tight text-gray-200">
                    Create an account
                </h2>
            </div>

            <div class="mt-10 sm:mx-auto sm:w-full sm:max-w-sm">
                <ActionForm action=action>
                    <CSRFField />
                    <div>
                        <label class="form-label">
                            Email address <div class="mt-2">
                                <input
                                    type="email"
                                    name="email"
                                    autocomplete="email"
                                    required
                                    class="form-input"
                                />
                            </div>
                        </label>
                    </div>
                    <div>
                        <label class="form-label">
                            Username <div class="mt-2">
                                <input
                                    type="text"
                                    name="username"
                                    autocomplete="username"
                                    required
                                    class="form-input"
                                />
                            </div>
                        </label>
                    </div>

                    <div>
                        <label class="form-label">
                            Password <div class="mt-2">
                                <input
                                    type="password"
                                    name="password"
                                    required
                                    class="form-input"
                                />
                            </div>
                        </label>

                    </div>
                    <div>
                        <label class="form-label">
                            Confirm Password <div class="mt-2">
                                <input
                                    type="password"
                                    name="password_confirmation"
                                    required
                                    class="form-input"
                                />
                            </div>
                        </label>
                    </div>

                    <div>
                        <label class="form-label flex">
                            Remember me <div class="flex h-6 shrink-0 items-center">
                                <div class="ml-2 group grid size-4 grid-cols-1">
                                    <input
                                        name="remember"
                                        type="checkbox"
                                        checked
                                        class="col-start-1 row-start-1 appearance-none rounded-sm border border-gray-300 bg-white checked:border-indigo-600 checked:bg-indigo-600 indeterminate:border-indigo-600 indeterminate:bg-indigo-600 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 disabled:border-gray-300 disabled:bg-gray-100 disabled:checked:bg-gray-100 forced-colors:appearance-auto"
                                    />
                                    <svg
                                        class="pointer-events-none col-start-1 row-start-1 size-3.5 self-center justify-self-center stroke-white group-has-disabled:stroke-gray-950/25"
                                        viewBox="0 0 14 14"
                                        fill="none"
                                    >
                                        <path
                                            class="opacity-0 group-has-checked:opacity-100"
                                            d="M3 8L6 11L11 3.5"
                                            stroke-width="2"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        />
                                        <path
                                            class="opacity-0 group-has-indeterminate:opacity-100"
                                            d="M3 7H11"
                                            stroke-width="2"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        />
                                    </svg>
                                </div>
                            </div>
                        </label>
                    </div>
                    <div>
                        <button type="submit" class="btn btn-primary">
                            Sign up
                        </button>
                    </div>
                    <div>{signup_result}</div>
                </ActionForm>
                <p class="mt-10 text-center text-sm/6 text-gray-500">
                    Already a member?
                    <A
                        href="/login"
                        attr:class="font-semibold text-indigo-600 hover:text-indigo-500"
                    >
                        Login
                    </A>
                </p>
            </div>
        </div>
    }
}

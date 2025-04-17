use leptos::{component, view, IntoView};
use leptos::prelude::{expect_context, Read};
use crate::app::ReadUserSignal;
use leptos::prelude::ElementChild;

#[component]
pub  fn UserInfoPage() -> impl IntoView{
    let user = expect_context::<ReadUserSignal>();
    view! {
        <h3>"User Info"</h3>
        <div>
            <p>
                "User: "
                {move || {
                    user.read().as_ref().map(|u| u.username.clone()).unwrap_or("None".to_string())
                }}
            </p>
            <p>
                "Email: "
                {move || {
                    user.read().as_ref().map(|u| u.email.clone()).unwrap_or("None".to_string())
                }}
            </p>
        </div>
    }
}

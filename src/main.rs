use {
    clyde_tools::{registry::ToolRegistry, ToolResult},
    indexmap::IndexMap,
    pyo3::{
        prelude::*,
        types::{IntoPyDict, PyDict, PyList, PyString},
    },
    std::{env, future::Future, pin::Pin, sync::Arc, time::Duration},
    tokio::time,
    twilight_cache_inmemory::DefaultInMemoryCache,
    twilight_gateway::{
        error::ReceiveMessageErrorType, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt,
    },
    twilight_http::Client,
    twilight_model::id::{marker::ChannelMarker, Id},
};

fn calculator(
    input: String,
) -> Pin<Box<dyn Future<Output = ToolResult<String>> + Send + Sync + 'static>> {
    Box::pin(async move { Ok(String::from("5")) })
}

fn load_url(
    input: String,
) -> Pin<Box<dyn Future<Output = ToolResult<String>> + Send + Sync + 'static>> {
    Box::pin(async move { Ok(String::from("5")) })
}

fn imagine(
    input: String,
) -> Pin<Box<dyn Future<Output = ToolResult<String>> + Send + Sync + 'static>> {
    Box::pin(async move { Ok(String::from("5")) })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    pyo3::prepare_freethreaded_python();
    tracing_subscriber::fmt::init();

    tokio::spawn(async {
        tokio::signal::ctrl_c().await;
        std::process::exit(1);
    });

    let token = env::var("DISCORD_TOKEN")?;
    let cache = DefaultInMemoryCache::builder()
        .message_cache_size(50)
        .build();

    let intents = Intents::DIRECT_MESSAGES | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;
    let mut shard = Shard::new(ShardId::ONE, token.clone(), intents);
    let client = Arc::new(Client::new(token));

    let mut tool_registry = ToolRegistry::builder();

    tool_registry.register(
        "calculator",
        "Evaluate the input.",
        Box::new(calculator)
            as Box<
                dyn FnMut(
                        String,
                    ) -> Pin<
                        Box<dyn Future<Output = ToolResult<String>> + Send + Sync + 'static>,
                    > + Send
                    + Sync
                    + 'static,
            >,
    )?;

    tool_registry.register(
        "load_url",
        "Load the specified URL",
        Box::new(load_url)
            as Box<
                dyn FnMut(
                        String,
                    ) -> Pin<
                        Box<dyn Future<Output = ToolResult<String>> + Send + Sync + 'static>,
                    > + Send
                    + Sync
                    + 'static,
            >,
    )?;

    tool_registry.register(
        "imagine",
        "Draw an image of the prompt",
        Box::new(imagine)
            as Box<
                dyn FnMut(
                        String,
                    ) -> Pin<
                        Box<dyn Future<Output = ToolResult<String>> + Send + Sync + 'static>,
                    > + Send
                    + Sync
                    + 'static,
            >,
    )?;

    let tool_registry = tool_registry.build("clyde_tools").map(Arc::new)?;

    while let Some(result) = shard.next_event(EventTypeFlags::all()).await {
        let event = match result {
            Ok(event) => event,
            Err(error) if is_fatal(error.kind()) => {
                tracing::error!("Fatal gateway error: {error}");

                return Err(error.into());
            }
            Err(error) => {
                tracing::warn!("Gateway error: {error}");

                continue;
            }
        };

        let (channel_id, author_id, guild_id) = match &event {
            Event::MessageCreate(message_create) => (
                message_create.channel_id,
                message_create.author.id,
                message_create.guild_id,
            ),
            Event::MessageDelete(message_delete) => {
                let Some(message) = cache.message(message_delete.id) else {
                    cache.update(&event);

                    continue;
                };

                (
                    message_delete.channel_id,
                    message.author(),
                    message_delete.guild_id,
                )
            }
            Event::MessageDeleteBulk(_message_delete_bulk) => {
                cache.update(&event);

                continue;

                // todo
            }
            Event::MessageUpdate(message_update) => {
                let Some(message) = cache.message(message_update.id) else {
                    cache.update(&event);

                    continue;
                };

                (
                    message_update.channel_id,
                    message.author(),
                    message_update.guild_id,
                )
            }
            _ => {
                cache.update(&event);

                continue;
            }
        };

        cache.update(&event);

        let whitelist = [
            1259779423381622795, /* #x */
            1244284242079514785, /* #general */
        ];

        if !whitelist.contains(&channel_id.get()) {
            continue;
        }

        let Some(current_user) = cache.current_user() else {
            continue;
        };

        if author_id == current_user.id {
            continue;
        }

        let Some(channel_messages) = cache.channel_messages(channel_id) else {
            continue;
        };

        fn tool_call<'py>(
            py: Python<'py>,
            name: &str,
            arg: &str,
            id: &str,
        ) -> PyResult<Bound<'py, PyAny>> {
            let tool_call = PyDict::new_bound(py);

            tool_call.set_item(pyo3::intern!(py, "name"), name)?;

            tool_call.set_item(
                pyo3::intern!(py, "args"),
                [(pyo3::intern!(py, "arg"), arg)].into_py_dict_bound(py),
            )?;

            tool_call.set_item(pyo3::intern!(py, "id"), id)?;

            Ok(tool_call.into_any())
        }

        let rules = [
            "You are a Discord user named Clyde",
            "Respond in lowercase, without punctuation, like an online chat user",
        ];

        let identity = rules.join(".\n") + ".";

        let messages = Python::with_gil(|py| {
            let messages = PyList::new_bound(
                py,
                [
                    python::langchain::system_message(py)?.call1((identity,))?,
                    /*python::langchain::user_message(py)?.call1(("kalmari246: hi clyde",))?,
                    python::langchain::assistant_message(py)?.call(
                        ("Clyde: sup, whats up",),
                        Some(&{
                            let kwargs = PyDict::new_bound(py);

                            kwargs.set_item(
                                pyo3::intern!(py, "name"),
                                pyo3::intern!(py, "example_assistanr"),
                            )?;

                            kwargs
                        }),
                    )?,
                    python::langchain::user_message(py)?.call1(("kalmari246: draw a cat",))?,
                    python::langchain::assistant_message(py)?.call(
                        ("",),
                        Some(&{
                            let kwargs = PyDict::new_bound(py);

                            kwargs.set_item(
                                pyo3::intern!(py, "name"),
                                pyo3::intern!(py, "example_assistanr"),
                            )?;

                            kwargs.set_item(
                                pyo3::intern!(py, "tool_calls"),
                                [tool_call(py, "imagine", "a cat", "1")?],
                            )?;

                            kwargs
                        }),
                    )?,*/
                ],
            )
            .unbind();

            Ok::<_, PyErr>(messages)
        })?;

        for message_id in channel_messages.iter().rev().take(10) {
            let Some(message) = cache.message(*message_id) else {
                tracing::error!("Missing cache entry for message {message_id}.");

                continue;
            };

            let author_id = message.author();

            let Some(author) = cache.user(author_id) else {
                tracing::error!("Missing cache entry for user {author_id}.");

                continue;
            };

            if let Some(guild_id) = guild_id {
                let Some(_member) = cache.member(guild_id, author_id) else {
                    tracing::error!(
                        "Missing cache entry for member {author_id} in guild {guild_id}."
                    );

                    continue;
                };
            }

            let content = format!("{}: {}", author.name, message.content());

            Python::with_gil(|py| {
                let messages = messages.bind(py);

                let message = if author.id == current_user.id {
                    python::langchain::assistant_message(py)
                } else {
                    python::langchain::user_message(py)
                };

                let message = message?.call1((content,))?.unbind();

                messages.append(message)?;

                Ok::<_, PyErr>(())
            })?;
        }

        let client = Arc::clone(&client);
        let tool_registry = Arc::clone(&tool_registry);

        let typing_future = tokio::spawn(type_forever(Arc::clone(&client), channel_id));
        let inference_future = tokio::spawn(async move {
            let content = Python::with_gil(|py| {
                let module = tool_registry.pydantic_module.bind(py);
                let messages = messages.bind(py);

                let tool_map = python::inspect::getmembers(py, module)?
                    .into_iter()
                    .flat_map(|pair| pair.extract::<(String, Bound<'_, PyAny>)>())
                    .filter(|(_name, member)| {
                        python::langchain::is_structured_tool(py, member).unwrap_or(false)
                    })
                    .collect::<IndexMap<_, _>>();

                let tool_list = PyList::new_bound(py, tool_map.values());

                let llm = python::langchain::ollama_functions(py)?
                    .call((), Some(&[("model", "gemma2")].into_py_dict_bound(py)))?
                    .getattr("bind_tools")?
                    .call1((tool_list,))?;

                let mut output = llm.getattr("invoke")?.call1((messages,))?;

                let tool_calls = output
                    .getattr("tool_calls")?
                    .downcast_into::<PyList>()
                    .map_err(PyErr::from)?;

                if !tool_calls.is_empty() {
                    let tool_calls = tool_calls
                        .into_iter()
                        .flat_map(|tool| tool.downcast_into::<PyDict>());

                    for tool in tool_calls {
                        let (Some(name), Some(args), Some(id)) = (
                            tool.get_item("name")?,
                            tool.get_item("args")?,
                            tool.get_item("id")?,
                        ) else {
                            continue;
                        };

                        let name = name.downcast_into::<PyString>().map_err(PyErr::from)?;
                        let name = name.extract::<&str>()?;

                        let Some(tool) = tool_map.get(name) else {
                            continue;
                        };

                        let tool_output = tool.getattr("invoke")?.call1((args,))?;
                        let tool_message = python::langchain::tool_message(py)?.call(
                            (tool_output,),
                            Some(&[("tool_call_id", id)].into_py_dict_bound(py)),
                        )?;

                        messages.append(tool_message)?;
                    }

                    output = llm.getattr("invoke")?.call1((messages,))?;
                }

                let content = output.getattr("content")?.extract::<String>()?;

                Ok::<_, anyhow::Error>(content)
            });

            let content = match content {
                Ok(content) if content.is_empty() => {
                    tracing::error!("no reply");

                    return Ok(());
                }
                Ok(content) => content,
                Err(error) => {
                    tracing::error!("{error}");

                    return Ok(());
                }
            };

            client.create_message(channel_id).content(&content).await?;

            Ok::<_, anyhow::Error>(())
        });

        tokio::pin!(inference_future);
        tokio::pin!(typing_future);

        loop {
            tokio::select! {
                result = &mut inference_future => {
                    result??;

                    break;
                },
                result = &mut typing_future => {
                    result??;

                    break;
                },
            }
        }
    }

    Ok(())
}

fn is_fatal(error_type: &ReceiveMessageErrorType) -> bool {
    matches!(
        error_type,
        ReceiveMessageErrorType::Reconnect | ReceiveMessageErrorType::WebSocket
    )
}

async fn type_forever(client: Arc<Client>, channel_id: Id<ChannelMarker>) -> anyhow::Result<()> {
    loop {
        client.create_typing_trigger(channel_id).await?;

        time::sleep(Duration::from_secs(10)).await;
    }
}

pub mod python {
    use pyo3::{prelude::*, sync::GILOnceCell, PyTypeCheck};

    pub(crate) fn lazy_import<'py, T>(
        cell: &'static GILOnceCell<Py<T>>,
        py: Python<'py>,
        module_name: &str,
        attr_name: &str,
    ) -> PyResult<&'py Bound<'py, T>>
    where
        T: PyTypeCheck,
    {
        cell.get_or_try_init(py, || {
            let value = py
                .import_bound(module_name)?
                .getattr(attr_name)?
                .downcast_into::<T>()
                .map_err(PyErr::from)?
                .unbind();

            Ok(value)
        })
        .map(|value| value.bind(py))
    }

    macro_rules! lazy_import {
        ($py:expr, $module_name:expr, $attr_name:expr, $pyty:ty $(,)?) => {{
            static LAZY: pyo3::sync::GILOnceCell<Py<$pyty>> = pyo3::sync::GILOnceCell::new();

            crate::python::lazy_import(&LAZY, $py, $module_name, $attr_name)
        }};
    }

    macro_rules! message {
        ($ident:ident, $attr_name:expr $(,)?) => {
            pub fn $ident(py: Python<'_>) -> PyResult<&Bound<'_, PyType>> {
                lazy_import!(py, "langchain_core.messages", $attr_name, PyType)
            }
        };
    }

    pub mod inspect {
        use pyo3::{
            prelude::*,
            types::{PyFunction, PyList},
        };

        pub fn getmembers(
            py: Python<'_>,
            object: impl IntoPy<Py<PyAny>>,
        ) -> PyResult<Bound<'_, PyList>> {
            lazy_import!(py, "inspect", "getmembers", PyFunction)?
                .call1((object,))?
                .downcast_into::<PyList>()
                .map_err(PyErr::from)
        }
    }

    pub mod langchain {
        use pyo3::{
            prelude::*,
            types::{PyAny, PyType},
        };

        pub fn is_structured_tool(
            py: Python<'_>,
            object: impl IntoPy<Py<PyAny>>,
        ) -> PyResult<bool> {
            object.into_py(py).bind(py).is_instance(lazy_import!(
                py,
                "langchain_core.tools",
                "StructuredTool",
                PyType,
            )?)
        }

        pub fn ollama_functions(py: Python<'_>) -> PyResult<&Bound<'_, PyAny>> {
            lazy_import!(
                py,
                "langchain_experimental.llms.ollama_functions",
                "OllamaFunctions",
                PyAny,
            )
        }

        message!(tool_message, "ToolMessage");
        message!(user_message, "HumanMessage");
        message!(system_message, "SystemMessage");
        message!(assistant_message, "AIMessage");
    }
}

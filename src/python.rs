use pyo3::{prelude::*, sync::GILOnceCell, PyTypeCheck};

/// Lazily import an attribute from a module.
pub(crate) fn lazy_import<'py, T: PyTypeCheck>(
    once_lock: &'static GILOnceCell<Py<T>>,
    py: Python<'py>,
    module_name: &str,
    attr_name: &str,
) -> PyResult<&'py Bound<'py, T>> {
    once_lock
        .get_or_try_init(py, || {
            let value = py
                .import_bound(module_name)?
                .getattr(attr_name)?
                .downcast_into::<T>()
                .map_err(PyErr::from)?
                .unbind();

            Ok::<_, PyErr>(value)
        })
        .map(|value| value.bind(py))
}

/// Lazily import an attribute from a module.
macro_rules! lazy_import {
    ($py:expr, $module_name:expr, $attr_name:expr, $ty:ty) => {{
        static LAZY: pyo3::sync::GILOnceCell<Py<$ty>> = pyo3::sync::GILOnceCell::new();

        crate::python::lazy_import(&LAZY, $py, $module_name, $attr_name)
    }};
}

pub mod langchain {
    use pyo3::{
        prelude::*,
        types::{IntoPyDict, PyDict, PyList},
    };
    use tokio::task;

    /// Determine which langchain message type to create.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Role {
        Assistant,
        System,
        Tool,
        User,
    }

    /// Return the langchain message type.
    fn try_into_py<'py>(role: Role, py: Python<'py>) -> PyResult<&'py Bound<'py, PyAny>> {
        match role {
            Role::Assistant => lazy_import!(py, "langchain_core.messages", "AIMessage", PyAny),
            Role::User => lazy_import!(py, "langchain_core.messages", "HumanMessage", PyAny),
            Role::System => lazy_import!(py, "langchain_core.messages", "SystemMessage", PyAny),
            Role::Tool => lazy_import!(py, "langchain_core.messages", "ToolMessage", PyAny),
        }
    }

    #[derive(Debug)]
    pub struct Message {
        role: Role,
        object: PyObject,
    }

    impl Message {
        fn new(role: Role, content: &str) -> Self {
            let result = Python::with_gil(|py| {
                let object = try_into_py(role, py)?
                    .call((), Some(&[("content", content)].into_py_dict_bound(py)))?
                    .unbind();

                Ok::<_, PyErr>(object)
            });

            let object = result.unwrap_or_else(|error| panic!("why: {error}"));

            Self { role, object }
        }

        pub fn assistant(content: &str) -> Self {
            Self::new(Role::Assistant, content)
        }

        pub fn system(content: &str) -> Self {
            Self::new(Role::System, content)
        }

        pub fn user(content: &str) -> Self {
            Self::new(Role::User, content)
        }
    }

    pub struct LangGraph {
        locals: Py<PyDict>,
    }

    impl LangGraph {
        pub fn new() -> Self {
            let result = Python::with_gil(|py| {
                let locals = PyDict::new_bound(py);

                py.run_bound(include_str!("graph.py"), None, Some(&locals))?;

                Ok::<_, PyErr>(locals.unbind())
            });

            let locals = result.unwrap_or_else(|_error| todo!());

            Self { locals }
        }

        pub fn invoke(
            &self,
            messages: impl IntoIterator<Item = Message>,
        ) -> anyhow::Result<String> {
            let content = Python::with_gil(|py| {
                let locals = self.locals.bind(py);
                let messages = messages
                    .into_iter()
                    .map(|message| message.object)
                    .collect::<Vec<_>>();

                locals.set_item("messages", PyList::new_bound(py, messages))?;

                tracing::error!("locals = {locals:?}");

                py.run_bound(
                    r#"output = graph.invoke({"messages": messages}, config={"configurable": {"thread_id": 42}})["messages"]"#,
                    None,
                    Some(&locals),
                )?;

                let output = locals
                    .get_item("output")?
                    .unwrap()
                    .downcast_into::<PyList>()
                    .map_err(PyErr::from)?;

                tracing::error!("output = {output:?}");

                let output = output
                    .get_item(output.len() - 1)?
                    .getattr("content")?
                    .extract::<String>()?
                    .trim()
                    .to_string();

                Ok::<_, PyErr>(output)
            });

            tracing::error!("result = {content:#?}");

            let content = content?;

            if content.is_empty() {
                return Err(anyhow::anyhow!("empty message"));
            }

            Ok(content)
        }

        pub async fn invoke_async(
            &self,
            messages: impl IntoIterator<Item = Message>,
        ) -> anyhow::Result<String> {
            let locals = Python::with_gil(|py| self.locals.clone_ref(py));
            let lang_graph = LangGraph { locals };
            let messages = messages.into_iter().collect::<Vec<_>>();

            task::spawn_blocking(move || lang_graph.invoke(messages)).await?
        }
    }
}

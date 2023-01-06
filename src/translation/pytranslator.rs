use camino::Utf8PathBuf;
use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use fastwave_backend::{Signal, SignalValue};
use pyo3::{types::PyModule, PyObject, Python, ToPyObject, PyResult, pymodule, pyfunction, wrap_pyfunction, pyclass, pymethods};

use super::{TranslationResult, Translator};

pub struct PyTranslator {
    name: String,
    instance: PyObject,
}

impl PyTranslator {
    pub fn new(name: &str, source: impl Into<Utf8PathBuf>) -> Result<Self> {
        let source = source.into();
        let code =
            std::fs::read_to_string(&source).with_context(|| format!("Failed to read {source}"))?;

        let instance = Python::with_gil(|py| -> Result<PyObject> {
            let module = PyModule::from_code(py, &code, source.as_str(), &name)
                .with_context(|| format!("Failed to load {name} in {source}"))?;

            let class = module.getattr(name)?;
            let instance = class.call0()?;

            let ensure_has_attr = |attr: &str| {
                if !instance.hasattr(attr)? {
                    bail!("Translator {name} does not have a `{attr}` method");
                }
                Ok(())
            };

            ensure_has_attr("translates")?;
            ensure_has_attr("translate")?;
            ensure_has_attr("signal_info")?;

            Ok(instance.to_object(py))
        })?;

        Ok(Self {
            name: name.to_string(),
            instance,
        })
    }
}

impl Translator for PyTranslator {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn translates(&self, name: &str) -> Result<bool> {
        Python::with_gil(|py| {
            let result = self
                .instance
                .call_method1(py, "translates", (name,))
                .with_context(|| format!("Failed to run translates on {}", self.name))?;

            Ok(result.extract(py)?)
        })
    }

    fn translate(&self, signal: &Signal, value: &SignalValue) -> Result<TranslationResult> {
        let value_str = match value {
            SignalValue::BigUint(val) => format!(
                "{val:0width$b}",
                width = signal.num_bits().unwrap_or(0) as usize
            ),
            SignalValue::String(val) => val.clone(),
        };

        Python::with_gil(|py| {
            let result = self
                .instance
                .call_method1(py, "translate", (signal.name(), value_str))
                .with_context(|| format!("Failed to run translates on {}", self.name))?;

            let val: PyTranslationResult = result.extract(py)?;
            Ok(val.0)
        })
    }
}

#[pyclass(name = "TranslationResult")]
#[derive(Clone)]
struct PyTranslationResult (TranslationResult);

#[pymethods]
impl PyTranslationResult {
    #[new]
    fn new(val_str: &str) -> Self {
        Self(TranslationResult {
            val: val_str.to_string(),
            subfields: vec![]
        })
    }
}

#[pyfunction]
fn test() {
    println!("test")
}

/// The python stuff we expose to python plugins. This must be apended to
/// the import stuff before python code is run, preferably by
/// ```
/// append_to_inittab!(surfer);
/// ```
/// early on in the program
#[pymodule]
pub fn surfer(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyTranslationResult>()?;
    m.add_function(wrap_pyfunction!(test, m)?)?;
    Ok(())
}

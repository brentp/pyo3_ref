use std::sync::{Arc, Mutex};
use pyo3::prelude::*;
use noodles::vcf;

#[pyclass]
struct InfoWrapper {
    record: Arc<Mutex<vcf::Record>>,
}

#[pymethods]
impl InfoWrapper {
    fn set(&self, key: &str, value: i32) -> PyResult<()> {
        let mut record = self.record.lock().unwrap();
        // Adjust according to the actual noodles API to set a field in Info.
        let val = vcf::record::info::field::Value::Integer(value);
        let key = vcf::record::info::field::Key::Other(key);
        record.info_mut().insert(key, Some(val));
        Ok(())
    }
}

#[pyclass]
struct RecordWrapper {
    record: Arc<Mutex<vcf::Record>>,
}

#[pymethods]
impl RecordWrapper {
    #[new]
    fn new() -> Self {
        RecordWrapper {
            record: Arc::new(Mutex::new(vcf::Record::default())),
        }
    }

    fn info(&self) -> InfoWrapper {
        InfoWrapper {
            record: Arc::clone(&self.record),
        }
    }
}

#[pymodule]
fn noodles_wrapper(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<RecordWrapper>()?;
    m.add_class::<InfoWrapper>()?;
    Ok(())
}

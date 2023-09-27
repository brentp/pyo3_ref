use noodles::vcf;
use pyo3::prelude::*;
use std::sync::Arc;

#[pyclass]
struct Variant {
    inner: Arc<vcf::Record>,
}

#[pyclass]
struct Info {
    variant: Py<Variant>,
}

#[pymethods]
impl Variant {
    #[getter]
    fn chromosome(&self) -> PyResult<String> {
        Ok(self.inner.chromosome().to_string())
    }

    #[getter]
    fn start(&self) -> PyResult<i64> {
        Ok((usize::from(self.inner.position()) - 1) as i64)
    }

    #[getter]
    fn stop(&self) -> PyResult<i64> {
        Ok((usize::from(self.inner.end().unwrap_or(self.inner.position())) - 1) as i64)
    }

    #[getter]
    fn info(&self, py: Python) -> PyResult<Py<Info>> {
        let info = Info {
            variant: Py::new(
                py,
                Variant {
                    inner: Arc::clone(&self.inner),
                },
            )?,
        };
        Py::new(py, info)
    }
}

#[pymethods]
impl Info {
    fn get(&self, key: &str, py: Python<'_>) -> PyResult<Option<String>> {
        let mut guard = self.variant.as_ref(py).borrow_mut();
        let variant = &mut *guard;

        let o: vcf::record::info::field::Key = match key.parse() {
            Ok(key) => key,
            Err(_) =>
            // return a python error
            {
                return Err(pyo3::exceptions::PyKeyError::new_err(format!(
                    "invalid info key: {}",
                    key
                )))
            }
        };

        Ok(variant.inner.info().get(&o).map(|v| format!("{:?}", v)))
    }
}

#[pymodule]
fn py_noodles_vcf(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Variant>()?;
    m.add_class::<Info>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_chromosome() {
        Python::with_gil(|py| {
            // write a test to check the chromosome
            let record = vcf::Record::default();
            let variant = Variant {
                inner: Arc::new(record),
            };
            let py_variant: Py<Variant> = Py::new(py, variant).unwrap();
            pyo3::py_run!(py, py_variant, "assert py_variant.chromosome == '.'");
        });
    }
}

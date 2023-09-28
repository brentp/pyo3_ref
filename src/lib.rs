use noodles::vcf;
use noodles::vcf::record::Chromosome;
use pyo3::prelude::*;

#[pyclass]
struct Variant {
    inner: vcf::Record,
}

#[pyclass]
struct Info {
    variant: Py<Variant>,
}

#[pymethods]
impl Variant {
    #[new]
    fn new() -> Self {
        Self {
            inner: vcf::Record::default(),
        }
    }
    #[getter]
    fn chromosome(&self) -> PyResult<String> {
        Ok(self.inner.chromosome().to_string())
    }

    #[setter(chromosome)]
    fn set_chromosome(&mut self, chromosome: &str) -> PyResult<()> {
        let c = self.inner.chromosome_mut();
        *c = Chromosome::Name(chromosome.to_string());
        Ok(())
    }

    #[getter]
    fn start(&self) -> i64 {
        usize::from(self.inner.position()) as i64 - 1
    }

    #[getter]
    fn stop(&self) -> i64 {
        // TODO: check off-by-one
        usize::from(self.inner.end().unwrap_or(self.inner.position())) as i64
    }

    fn clone_me(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[getter]
    fn info(&self, py: Python<'_>) -> PyResult<Py<Info>> {
        // Q: how to get Py<Variant> from &mut Variant here?
        let v: Py<Variant> = Py::new(py, self)?;
        //                   ^^^^^^^^^^^^^^^^^^ expected `Py<Variant>`, found `Py<&Variant>`

        let clone = Variant::clone_me(v);
        let info = Info { variant: clone };
        Py::new(py, info)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Variant(chromosome={}, start={}, stop={})",
            self.chromosome()?,
            self.start(),
            self.stop()
        ))
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
fn pyo3_ref(_py: Python, m: &PyModule) -> PyResult<()> {
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
            let variant = Variant {
                inner: vcf::Record::default(),
            };
            let py_variant: Py<Variant> = Py::new(py, variant).unwrap();
            pyo3::py_run!(
                py,
                py_variant,
                "py_variant.chromosome = 'chr22'; assert py_variant.chromosome == 'chr22'"
            );
        });
    }
}

use noodles::vcf;
use noodles::vcf::record::{info, position, Chromosome};
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
    #[pyo3(signature = (chromosome="chr1", start=0, reference="A", alternate="T", info=None))]
    fn new(
        chromosome: &str,
        start: usize,
        reference: &str,
        alternate: &str,
        info: Option<&str>,
    ) -> PyResult<Self> {
        let mut b = vcf::Record::builder()
            .set_chromosome(match chromosome.parse() {
                Ok(chromosome) => chromosome,
                Err(_) => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "invalid chromosome: {}",
                        chromosome
                    )))
                }
            })
            .set_position(match position::Position::try_from(start + 1) {
                Ok(position) => position,
                Err(_) => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "invalid start: {}",
                        start
                    )))
                }
            })
            .set_reference_bases(match reference.parse() {
                Ok(reference) => reference,
                Err(_) => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "invalid reference: {}",
                        reference
                    )))
                }
            })
            .set_alternate_bases(match alternate.parse() {
                Ok(alt) => alt,
                Err(_) => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "invalid reference: {}",
                        alternate
                    )))
                }
            });
        if let Some(info) = info {
            b = b.set_info(match info.parse() {
                Ok(info) => info,
                Err(_) => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "invalid info: {}",
                        info
                    )))
                }
            });
        }
        let b = b.build().unwrap();
        Ok(Self { inner: b })
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

    #[getter]
    fn info(slf: Py<Self>, py: Python<'_>) -> PyResult<Py<Info>> {
        let info = Info { variant: slf };
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

#[pyclass]
pub struct InfoValue(info::field::Value);

#[pymethods]
impl InfoValue {
    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.0))
    }

    fn integer(&self) -> PyResult<i32> {
        match self.0 {
            info::field::Value::Integer(i) => Ok(i),
            _ => Err(pyo3::exceptions::PyTypeError::new_err("not an integer")),
        }
    }
}

impl From<info::field::Value> for InfoValue {
    fn from(value: info::field::Value) -> Self {
        Self(value)
    }
}

#[pymethods]
impl Info {
    fn get(&self, key: &str, py: Python<'_>) -> Option<InfoValue> {
        let mut guard = self.variant.as_ref(py).borrow_mut();
        let variant = &mut *guard;

        let o: info::field::Key = match key.parse() {
            Ok(key) => key,
            Err(_) => return None,
        };
        match variant.inner.info().get(&o) {
            None => None,
            Some(info) => match info {
                None => None,
                Some(info) => Some(info.clone().into()),
            },
        }
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let mut guard = self.variant.as_ref(py).borrow_mut();
        let variant = &mut *guard;
        Ok(format!("{:?}", variant.inner.info()))
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
    use pyo3::types::IntoPyDict;

    #[test]
    fn test_get_chromosome() {
        Python::with_gil(|py| {
            // write a test to check the chromosome
            let locals = [
                ("Variant", py.get_type::<Variant>()),
                ("InfoValue", py.get_type::<InfoValue>()),
            ]
            .into_py_dict(py);

            pyo3::py_run!(
                py,
                *locals,
                r#"
                v = Variant(chromosome='chrX', info='DP=10')
                print(v)
                print(v.info.get('DP'))
                assert v.info.get('DP').integer() == 10
                #assert v.chromosome == 'chrX'
                #v.chromosome = 'chr22'; 
                #assert v.chromosome == 'chr22'
                #info = v.info
                #assert v.chromosome == 'chr22'
                #print(info)

                #print(info.get('DP'))
                
                "#
            );
        });
    }
}

use noodles::vcf;
use noodles::vcf::record::{info, position, Chromosome};
use pyo3::prelude::*;
use xvcf;

#[pyclass]
struct Variant {
    inner: vcf::Record,
}

#[pyclass]
struct VCF {
    inner: xvcf::Reader,
    header: vcf::Header,
}

#[pyclass]
struct Info {
    variant: Py<Variant>,
}

#[pymethods]
impl VCF {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let inner = xvcf::Reader::from_path(path).unwrap();
        let header = inner.header().clone();
        Ok(Self { inner, header })
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Variant> {
        let mut record = vcf::record::Record::default();
        let header = slf.header.clone();
        match slf.inner.next_record(&header, &mut record) {
            Ok(0) => None,
            Ok(_) => Some(Variant { inner: record }),
            Err(_) => None,
        }
    }
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

    #[staticmethod]
    fn from_integer(i: i32) -> InfoValue {
        InfoValue(info::field::Value::Integer(i))
    }

    #[staticmethod]
    fn from_float(f: f32) -> InfoValue {
        InfoValue(info::field::Value::Float(f))
    }

    #[staticmethod]
    fn from_string(s: &str) -> InfoValue {
        InfoValue(info::field::Value::String(s.to_string()))
    }

    fn integer(&self) -> PyResult<i32> {
        match self.0 {
            info::field::Value::Integer(i) => Ok(i),
            _ => Err(pyo3::exceptions::PyTypeError::new_err("not an integer")),
        }
    }

    fn float(&self) -> PyResult<f32> {
        match self.0 {
            info::field::Value::Float(f) => Ok(f),
            info::field::Value::Integer(i) => Ok(i as f32),
            _ => Err(pyo3::exceptions::PyTypeError::new_err("not a float")),
        }
    }

    fn string(&self) -> PyResult<String> {
        match &self.0 {
            info::field::Value::String(s) => Ok(s.to_string()),
            _ => Err(pyo3::exceptions::PyTypeError::new_err("not a string")),
        }
    }
}

impl From<info::field::Value> for InfoValue {
    fn from(value: info::field::Value) -> Self {
        Self(value)
    }
}
impl From<InfoValue> for info::field::Value {
    fn from(value: InfoValue) -> Self {
        value.0
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

    fn set(&mut self, key: &str, value: &InfoValue, py: Python<'_>) -> PyResult<()> {
        let mut guard = self.variant.as_ref(py).borrow_mut();
        let variant = &mut *guard;

        let o: info::field::Key = match key.parse() {
            Ok(key) => key,
            Err(_) => {
                return Err(pyo3::exceptions::PyKeyError::new_err(format!(
                    "invalid key: {}",
                    key
                )))
            }
        };
        let v: info::field::Value = value.0.clone();
        variant.inner.info_mut().insert(o, Some(v));
        Ok(())
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

                v.info.set('DP', InfoValue.from_integer(99))
                assert v.info.get('DP').integer() == 99

                v.info.set('DP', InfoValue.from_string("hello"))
                print(v.info.get('DP'))
                

                
                "#
            );
        });
    }

    #[test]
    fn test_read_file() {
        Python::with_gil(|py| {
            // write a test to check the chromosome
            let locals = [("VCF", py.get_type::<VCF>())].into_py_dict(py);

            pyo3::py_run!(
                py,
                *locals,
                r#"
                vcf = VCF('tests/t.vcf.gz')
                for variant in vcf:
                    print(variant.info.get('DP'))
                    print(variant.info.get('DP').integer())
                "#
            );
        });
    }
}

use crate::SimpleSerialize;
use thiserror::Error;

// NOTE: if this is changed, go change in `ssz_derive` as well!
pub const BYTES_PER_LENGTH_OFFSET: usize = 4;
const MAXIMUM_LENGTH: u64 = 2u64.pow((8 * BYTES_PER_LENGTH_OFFSET) as u32);

#[derive(Error, Debug)]
#[error("the value could not be serialized: {0}")]
pub enum SerializeError {
    #[error("the encoded length is {0} which exceeds the maximum length {MAXIMUM_LENGTH}")]
    MaximumEncodedLengthExceeded(usize),
    #[error("the type for this value has a bound of {bound} but the value has {len} elements")]
    TypeBoundsViolated { bound: usize, len: usize },
    #[error("the type for this value has an illegal bound of {bound}")]
    IllegalType { bound: usize },
}

pub trait Serialize {
    /// Append an encoding of `self` to the `buffer`.
    /// Return the number of bytes written.
    fn serialize(&self, buffer: &mut Vec<u8>) -> Result<usize, SerializeError>;
}

pub fn serialize_composite_from_components(
    mut fixed: Vec<Option<Vec<u8>>>,
    mut variable: Vec<Vec<u8>>,
    variable_lengths: Vec<usize>,
    fixed_lengths_sum: usize,
    buffer: &mut Vec<u8>,
) -> Result<usize, SerializeError> {
    let total_size = fixed_lengths_sum + variable_lengths.iter().sum::<usize>();
    if total_size as u64 >= MAXIMUM_LENGTH {
        return Err(SerializeError::MaximumEncodedLengthExceeded(total_size))
    }

    let mut total_bytes_written = 0;

    for (i, part_opt) in fixed.iter_mut().enumerate() {
        if let Some(part) = part_opt {
            total_bytes_written += part.len();
            buffer.append(part);
        } else {
            let variable_lengths_sum = variable_lengths[0..i].iter().sum::<usize>();
            let length = (fixed_lengths_sum + variable_lengths_sum) as u32;
            let mut offset_buffer = Vec::with_capacity(4);
            let _ = length.serialize(&mut offset_buffer).expect("can serialize simple type");
            buffer.append(&mut offset_buffer);
            total_bytes_written += 4;
        }
    }

    for part in variable.iter_mut() {
        total_bytes_written += part.len();
        buffer.append(part);
    }

    Ok(total_bytes_written)
}

pub fn serialize_composite<T: SimpleSerialize>(
    elements: &[T],
    buffer: &mut Vec<u8>,
) -> Result<usize, SerializeError> {
    let mut fixed = vec![];
    let mut variable = vec![];
    let mut variable_lengths = vec![];
    let mut fixed_lengths_sum = 0;

    for element in elements {
        let mut buffer = Vec::with_capacity(T::size_hint());
        element.serialize(&mut buffer)?;

        let buffer_len = buffer.len();
        if T::is_variable_size() {
            fixed.push(None);
            fixed_lengths_sum += BYTES_PER_LENGTH_OFFSET;
            variable.push(buffer);
            variable_lengths.push(buffer_len);
        } else {
            fixed.push(Some(buffer));
            fixed_lengths_sum += buffer_len;
            variable_lengths.push(0)
        }
    }

    serialize_composite_from_components(
        fixed,
        variable,
        variable_lengths,
        fixed_lengths_sum,
        buffer,
    )
}

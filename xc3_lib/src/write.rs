use binrw::{BinResult, BinWrite};

pub(crate) trait Xc3Write {
    type Offsets<'a>
    where
        Self: 'a;

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets<'_>>;

    // TODO: Look at pointers to determine default alignment.
    const ALIGNMENT: u64 = 1;
}

// Support importing both the trait and derive macro at once.
pub(crate) use xc3_lib_derive::Xc3Write;

pub(crate) struct Offset<'a, T> {
    /// The position in the file for the offset field.
    pub position: u64,
    /// The data pointed to by the offset.
    pub data: &'a T,
    /// Additional alignment applied at the field level.
    /// This may be stricter than the alignment of `T`.
    pub field_alignment: u64,
}

impl<'a, T: Xc3Write> std::fmt::Debug for Offset<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Don't print the actual data to avoid excessive output.
        f.debug_struct("Offset")
            .field("position", &self.position)
            .field("data", &std::any::type_name::<T>())
            .finish()
    }
}

impl<'a, T> Offset<'a, T> {
    pub fn new(position: u64, data: &'a T, field_alignment: u64) -> Self {
        Self {
            position,
            data,
            field_alignment,
        }
    }
}

impl<'a, T: Xc3Write> Offset<'a, T> {
    // TODO: make the data ptr u32?
    // TODO: Specify an alignment using another trait?
    pub(crate) fn write_offset<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr_base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<T::Offsets<'_>> {
        // Account for the type and field alignment.
        *data_ptr = round_up(*data_ptr, T::ALIGNMENT);
        *data_ptr = round_up(*data_ptr, self.field_alignment);

        // Update the offset value.
        writer.seek(std::io::SeekFrom::Start(self.position))?;
        ((*data_ptr - data_ptr_base_offset) as u32).write_le(writer)?;

        // Write the data.
        writer.seek(std::io::SeekFrom::Start(*data_ptr))?;
        let offsets = self.data.write(writer, data_ptr)?;

        Ok(offsets)
    }
}

impl<'a, T: Xc3Write> Offset<'a, Option<T>> {
    pub(crate) fn write_offset<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr_base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<Option<T::Offsets<'_>>> {
        // Only update the offset if there is data.
        if let Some(data) = self.data {
            // Update the offset value.
            writer.seek(std::io::SeekFrom::Start(self.position))?;
            *data_ptr = round_up(*data_ptr, T::ALIGNMENT);
            ((*data_ptr - data_ptr_base_offset) as u32).write_le(writer)?;

            // Write the data.
            writer.seek(std::io::SeekFrom::Start(*data_ptr))?;
            let offsets = data.write(writer, data_ptr)?;
            Ok(Some(offsets))
        } else {
            Ok(None)
        }
    }
}

macro_rules! xc3_write_binwrite_impl {
    ($($ty:ty),*) => {
        $(
            impl Xc3Write for $ty {
                type Offsets<'a> = ();

                fn write<W: std::io::Write + std::io::Seek>(
                    &self,
                    writer: &mut W,
                    data_ptr: &mut u64,
                ) -> BinResult<Self::Offsets<'_>> {
                    self.write_le(writer)?;
                    *data_ptr = (*data_ptr).max(writer.stream_position()?);
                    Ok(())
                }
            }
        )*

    };
}

pub(crate) use xc3_write_binwrite_impl;

// TODO: Add alignment as a parameter.
xc3_write_binwrite_impl!(u8, u16);

// TODO: Macro for implementing for binwrite?
impl Xc3Write for String {
    type Offsets<'a> = ();

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets<'_>> {
        self.as_bytes().write_le(writer)?;
        0u8.write_le(writer)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(())
    }

    const ALIGNMENT: u64 = 1;
}

impl<T> Xc3Write for Vec<T>
where
    T: Xc3Write + 'static,
{
    type Offsets<'a> = Vec<T::Offsets<'a>>;

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets<'_>> {
        let result = self.iter().map(|v| v.write(writer, data_ptr)).collect();
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        result
    }
}

pub(crate) const fn round_up(x: u64, n: u64) -> u64 {
    ((x + n - 1) / n) * n
}

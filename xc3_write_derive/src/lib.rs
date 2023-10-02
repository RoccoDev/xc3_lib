use attribute::{FieldOptions, FieldType, TypeOptions};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, Ident, Type};

mod attribute;

#[proc_macro_derive(Xc3Write, attributes(xc3))]
pub fn xc3_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let offsets_name = offsets_name(&input.ident);

    let fields = parse_field_data(&input.data);

    let options = TypeOptions::from_attrs(&input.attrs);

    // Some types need a pointer to the start of the type.
    let base_offset_field = options
        .has_base_offset
        .then_some(quote!(pub base_offset: u64,));
    let base_offset = options.has_base_offset.then_some(quote!(base_offset,));
    let set_base_offset = options
        .has_base_offset
        .then_some(quote!(let base_offset = writer.stream_position()?;));

    let write_magic = options.magic.map(|m| quote!(#m.write_le(writer)?;));

    let offset_fields = fields.iter().map(|f| &f.offset_field);
    let offsets_struct = quote! {
        #[doc(hidden)]
        pub struct #offsets_name<'a> {
            #base_offset_field
            #(#offset_fields),*
        }
    };

    let offset_field_names = fields.iter().map(|f| &f.name);
    let initialize_offsets_struct = quote! {
        Ok(#offsets_name { #base_offset #(#offset_field_names),* })
    };

    let offsets_type = quote!(#offsets_name<'a>);

    // TODO: move offset struct generation to the field data?
    let write_fields = fields.iter().map(|f| &f.write_impl);
    quote! {
        #offsets_struct

        impl ::xc3_write::Xc3Write for #name {
            type Offsets<'a> = #offsets_type;

            fn xc3_write<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                data_ptr: &mut u64,
            ) -> binrw::BinResult<Self::Offsets<'_>> {
                use binrw::BinWrite;
                #set_base_offset

                #write_magic

                // Write data and placeholder offsets.
                #(#write_fields)*

                // Point past current write.
                *data_ptr = (*data_ptr).max(writer.stream_position()?);

                // Return positions of offsets to update later.
                #initialize_offsets_struct
            }
        }
    }
    .into()
}

// Share attributes with Xc3Write.
#[proc_macro_derive(Xc3WriteOffsets, attributes(xc3))]
pub fn xc3_write_offsets_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let offsets_name = offsets_name(&input.ident);

    let fields = parse_field_data(&input.data);

    let options = TypeOptions::from_attrs(&input.attrs);
    let self_base_offset = if options.has_base_offset {
        quote!(self.base_offset;)
    } else {
        quote!(base_offset)
    };

    // The offsets are the last thing to be written.
    // Final alignment should go here instead of Xc3Write.
    // TODO: Share logic with pad_size_to?
    let align_after = options.align_after.map(|align| {
        quote! {
            // Round up the total size.
            let size = writer.stream_position()?;
            let round_up = |x, n| ((x + n - 1) / n) * n;
            let desired_size = round_up(size, #align);
            let padding = desired_size - size;
            writer.write_all(&vec![0u8; padding as usize])?;

            // Point past current write.
            *data_ptr = (*data_ptr).max(writer.stream_position()?);
        }
    });

    // Add a write impl to the offset type to support nested types.
    // Vecs need to be able to write all items before the pointed to data.
    let write_offset_fields = fields.iter().map(|f| &f.write_offset_impl);
    quote! {
        impl<'a> ::xc3_write::Xc3WriteOffsets for #offsets_name<'a> {
            fn write_offsets<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                base_offset: u64,
                data_ptr: &mut u64,
            ) -> binrw::BinResult<()> {
                // Assume data is arranged in order by field.
                // TODO: investigate deriving other orderings.
                let base_offset = #self_base_offset;
                #(#write_offset_fields)*

                #align_after

                Ok(())
            }
        }
    }
    .into()
}

fn offsets_name(ident: &Ident) -> Ident {
    Ident::new(&(ident.to_string() + "Offsets"), Span::call_site())
}

// Collect writing related information and code for each field.
struct FieldData {
    name: Ident,
    offset_field: TokenStream2,
    write_impl: TokenStream2,
    write_offset_impl: TokenStream2,
}

impl FieldData {
    fn offset(name: &Ident, alignment: Option<u64>, pointer: &TokenStream2, ty: &Type) -> Self {
        Self {
            name: name.clone(),
            offset_field: offset_field(name, pointer, ty),
            write_impl: write_dummy_offset(name, alignment, pointer),
            write_offset_impl: quote! {
                self.#name.write_full(writer, base_offset, data_ptr)?;
            },
        }
    }

    fn shared_offset(name: &Ident, alignment: Option<u64>, pointer: &TokenStream2) -> Self {
        Self {
            name: name.clone(),
            offset_field: offset_field(name, pointer, &Type::Verbatim(quote!(()))),
            write_impl: write_dummy_shared_offset(name, alignment, pointer),
            write_offset_impl: quote! {
                self.#name.write_full(writer, base_offset, data_ptr)?;
            },
        }
    }
}

fn write_dummy_offset(
    name: &Ident,
    alignment: Option<u64>,
    pointer: &TokenStream2,
) -> TokenStream2 {
    let alignment = match alignment {
        Some(align) => quote!(Some(#align)),
        None => quote!(None),
    };
    quote! {
        let #name = ::xc3_write::Offset::new(writer.stream_position()?, &self.#name, #alignment);
        // Assume 0 is the default for the pointer type.
        #pointer::default().write_le(writer)?;
    }
}

fn write_dummy_shared_offset(
    name: &Ident,
    alignment: Option<u64>,
    pointer: &TokenStream2,
) -> TokenStream2 {
    let alignment = match alignment {
        Some(align) => quote!(Some(#align)),
        None => quote!(None),
    };
    quote! {
        let #name = ::xc3_write::Offset::new(writer.stream_position()?, &(), #alignment);
        // Assume 0 is the default for the pointer type.
        #pointer::default().write_le(writer)?;
    }
}

fn parse_field_data(data: &Data) -> Vec<FieldData> {
    let mut offset_fields = Vec::new();

    match data {
        syn::Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            for f in fields.named.iter() {
                let name = f.ident.as_ref().unwrap();
                let ty = &f.ty;

                let options = FieldOptions::from_attrs(&f.attrs);

                let pad_size_to = options.pad_size_to.map(|desired_size| {
                    quote! {
                        // Add appropriate padding until desired size.
                        let after_pos = writer.stream_position()?;
                        let size = after_pos - before_pos;
                        let padding = #desired_size - size;
                        writer.write_all(&vec![0u8; padding as usize])?;

                        // Point past current write.
                        *data_ptr = (*data_ptr).max(writer.stream_position()?);
                    }
                });

                // Check if we need to write the count.
                // Use a null offset as a placeholder.
                // TODO: Reduce repeated code?
                match options.field_type {
                    Some(FieldType::Offset16) => {
                        offset_fields.push(FieldData::offset(
                            name,
                            options.align,
                            &quote!(u16),
                            ty,
                        ));
                    }
                    Some(FieldType::Offset32) => {
                        offset_fields.push(FieldData::offset(
                            name,
                            options.align,
                            &quote!(u32),
                            ty,
                        ));
                    }
                    Some(FieldType::Offset64) => {
                        offset_fields.push(FieldData::offset(
                            name,
                            options.align,
                            &quote!(u64),
                            ty,
                        ));
                    }
                    Some(FieldType::Count32Offset32) => {
                        let write_offset = write_dummy_offset(name, options.align, &quote!(u32));

                        offset_fields.push(FieldData {
                            name: name.clone(),
                            offset_field: offset_field(name, &quote!(u32), ty),
                            write_impl: quote! {
                                (self.#name.len() as u32).write_le(writer)?;
                                #write_offset
                            },
                            write_offset_impl: quote! {
                                self.#name.write_full(writer, base_offset, data_ptr)?;
                            },
                        });
                    }
                    Some(FieldType::Offset32Count32) => {
                        let write_offset = write_dummy_offset(name, options.align, &quote!(u32));

                        offset_fields.push(FieldData {
                            name: name.clone(),
                            offset_field: offset_field(name, &quote!(u32), ty),
                            write_impl: quote! {
                                #write_offset
                                (self.#name.len() as u32).write_le(writer)?;
                            },
                            write_offset_impl: quote! {
                                self.#name.write_full(writer, base_offset, data_ptr)?;
                            },
                        });
                    }
                    Some(FieldType::SharedOffset) => {
                        // Shared offsets don't actually contain any data.
                        // The pointer type is the type of the field itself.
                        offset_fields.push(FieldData::shared_offset(
                            name,
                            options.align,
                            &quote!(#ty),
                        ));
                    }
                    None => {
                        // Also include fields not marked as offsets in the struct.
                        // The field type may have offsets that need to be written later.
                        let write_impl = if options.pad_size_to.is_some() {
                            quote! {
                                let before_pos = writer.stream_position()?;
                                let #name = self.#name.xc3_write(writer, data_ptr)?;
                                #pad_size_to
                            }
                        } else {
                            quote! {
                                let #name = self.#name.xc3_write(writer, data_ptr)?;
                            }
                        };
                        offset_fields.push(FieldData {
                            name: name.clone(),
                            offset_field: quote!(pub #name: <#ty as ::xc3_write::Xc3Write>::Offsets<'a>),
                            write_impl,
                            write_offset_impl: quote! {
                                // This field isn't an Offset<T>, so just call write_offsets.
                                self.#name.write_offsets(writer, base_offset, data_ptr)?;
                            },
                        });
                    }
                }
            }
        }
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
        _ => panic!("Unsupported type"),
    }

    offset_fields
}

fn offset_field(name: &Ident, pointer: &TokenStream2, ty: &Type) -> TokenStream2 {
    quote!(pub #name: ::xc3_write::Offset<'a, #pointer, #ty>)
}

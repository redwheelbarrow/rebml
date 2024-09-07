# Rebml
A library to support writing and reading EBML files in dynamic environments.

# Goals
1. Read and write simultaneously using the same in memory data structure
1. Leave file/IO management to the client code
1. Minimal memory footprint
1. Leave schema adherence to higher level libraries
1. Enable low level control of bytes

# Read Example
```
fn main() -> Result<(), EbmlError> {
  let file = File::open("test.mkv").unwrap();
  let data = unsafe { Mmap::map(&file).unwrap() };
  let mut cursor = Cursor::new(&data[..]);

  let header = EbmlHeader::try_from(&mut cursor)?;
  println!("{header:#?}");
  
  let body = EbmlElement::try_from(&mut cursor)?;
  println!("{:X}, {}", body.id, body.size.value);
  match body.id {
    0x18538067 => {
      println!("This is a matroska segment");
      let data = body.get_child(&mut cursor)?;
      println!("First child: {:X}", data.id);
    },
    _ => println!("{:X}", body.id),
  }
}
```
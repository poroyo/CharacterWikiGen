use anyhow::{anyhow, Error};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use bytes::{Buf, BytesMut};

pub struct Chunk {
    chunk_type: String,
    chunk_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Character {
    pub name: String,
    pub personality: String,
    pub description: String,
}

pub fn read_chunks(data: &[u8]) -> Result<Vec<Chunk>, Error> {
    let mut vec_chunks = Vec::new();
    let mut buf = BytesMut::from(data);

    // Signature 읽기 (8 bytes)
    let signature = buf.split_to(8);

    // 시그니쳐 체크
    if signature.as_ref() != [137, 80, 78, 71, 13, 10, 26, 10] {
        return Err(anyhow!("Invalid PNG Signature"));
    }

    // Chunk들 읽기
    while buf.has_remaining() {
        // Length 읽기 (4 bytes)
        let length = buf.get_u32() as usize;

        // Chunk Type 읽기 (4 bytes)
        let chunk_type = buf.split_to(4);
        let chunk_type_str = String::from_utf8_lossy(&chunk_type);

        // Chunk Data 읽기 (Length bytes)
        let chunk_data = buf.split_to(length);

        // CRC 읽기 (4 bytes)
        let crc = buf.get_u32();

        // CRC 체크
        let mut haser = crc32fast::Hasher::new();
        haser.update(&chunk_type);
        haser.update(&chunk_data);
        if crc != haser.finalize() {
            return Err(anyhow!(
                "CRC for {} is invalid",
                chunk_type_str
            ));
        }

        let chunk = Chunk {
            chunk_type: chunk_type_str.into(),
            chunk_data: chunk_data.to_vec(),
        };
        vec_chunks.push(chunk);
    }

    Ok(vec_chunks)
}

// 청크가 IHDR로 시작하고 IEND로 끝나는지 확인
pub fn check_vaild(vec_chunks: &Vec<Chunk>) -> Result<(), Error> {
    if vec_chunks[0].chunk_type != "IHDR" {
        return Err(anyhow!("missing IHDR header"));
    }
    if vec_chunks[vec_chunks.len() - 1].chunk_type != "IEND" {
        return Err(anyhow!("missing IEND header"));
    }
    Ok(())
}

fn parsing_data(data: Chunk) -> Result<String, Error> {
    // null 기준으로 자르고 앞부분이 chara로 시작하는지 확인
    let mut data = data.chunk_data.split(|&x| x == 0);
    if "chara" != String::from_utf8_lossy(data.next().unwrap()) {
        return Err(anyhow!(
            "It's not a character card, or it's an invalid character card."
        ));
    }

    // 데이터를 문자열로 바꾸기
    let text = String::from_utf8_lossy(data.next().unwrap());

    // base64로 인코딩된 문자열을 유니코드로 변환
    let mut buffer = Vec::<u8>::new();
    general_purpose::STANDARD.decode_vec(&text[..], &mut buffer)?;
    let contents = String::from_utf8(buffer)?;
    Ok(contents)
}

// tEXt를 필터링하고 내보냄
pub fn parsing_text(vec_chunks: Vec<Chunk>) -> Option<String> {
    vec_chunks
    .into_iter()
    .filter(|v| v.chunk_type == "tEXt")
    .map(parsing_data)
    .filter_map(Result::ok)
    .next()
} 

pub fn parsing_text_for_cat(text: Character) -> (String, String, String) {
    let Character { name, description, personality } = text;
    let description = description.replace(r#"\r\n"#, "\n");
    (name,  personality, description)
}

#[test]
fn test_something() -> Result<(), Error> {
    use std::io::Read;
    let mut reader = std::fs::File::open("abc.png").unwrap();
    let mut contents = Vec::new();
    reader.read_to_end(&mut contents).unwrap();
    let vec_chunks = read_chunks(contents.as_slice())
        .map_err(|_| anyhow!("유효하지 않은 캐릭터 카드입니다."))?;
    check_vaild(&vec_chunks).map_err(|_| anyhow!("유효하지 않은 캐릭터 카드입니다."))?;
    let script = parsing_text(vec_chunks);
    // "tEXt" 가 없으면 에러
    if script.is_none() {
        return Err(anyhow!("유효하지 않은 캐릭터 카드입니다."));
    }
    println!("{}", script.unwrap());
    Ok(())
}

use byteorder::{NetworkEndian, WriteBytesExt, ReadBytesExt};
use std::io::{Seek, SeekFrom, Write, Read};
use leb128;
use crate::common::fnv1a32;
use crate::common::*;
use num::{FromPrimitive, ToPrimitive};

/////////////////////////////////
// Basic Types/Functions

type FileId = u64;

pub enum AppTlvParseResult {
    Ok(AppTlv),
    Err(ReadError)
}

pub fn parse(cursor: &mut Cursor) -> AppTlvParseResult {
    let app_tlv_type = cursor.read_u8().unwrap();
    cursor.seek(SeekFrom::Current(-1)).expect("Seek back (2) failed."); // TLV will read type again.
    let mut app_tlv = match FromPrimitive::from_u8(app_tlv_type) {
        Some(AppTlvType::FileRequest) => AppTlv::FileRequest(FileRequest::default()),
        Some(AppTlvType::FileResume) => AppTlv::FileResume(FileResume::default()),
        Some(AppTlvType::FileMetadata) => AppTlv::FileMetadata(FileMetadata::default()),
        Some(AppTlvType::FileContent) => AppTlv::FileContent(FileContent::default()),
        Some(AppTlvType::ApplicationError) => AppTlv::ApplicationError(ApplicationError::default()),
        Some(AppTlvType::FileListRequest) => AppTlv::FileListRequest(FileListRequest::default()),
        Some(AppTlvType::FileListResponse) => AppTlv::FileListResponse(FileListResponse::default()),
        None => return AppTlvParseResult::Err(
            ReadError::new(
                format!("Unknown application message type code {}!", app_tlv_type).as_str())),
    };
    match app_tlv.read(cursor) {
        ReadResult::Ok => AppTlvParseResult::Ok(app_tlv),
        ReadResult::Err(e) => AppTlvParseResult::Err(e)
    }
}

/////////////////////////////////
// Object Types

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq)]
#[repr(u8)]
pub enum ObjectType {
    FileRequest = 0x10,
    FileResponse = 0x11,
    ErrorReport = 0x12,
    FileListRequest = 0x13,
    FileListResponse = 0x14,
}

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq)]
#[repr(u8)]
pub enum ObjectFieldType {
    FileRequestSend = 0x20,
    FileRequestResume = 0x21,
    FileResponseMetadata = 0x22,
    FileResponseContent = 0x23,
    ErrorReportContent = 0x24,
    FileListRequestContent = 0x25,
    FileListResponseContent = 0x26
}

/////////////////////////////////
// AppTlv

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq)]
#[repr(u8)]
pub enum AppTlvType {
    FileRequest = 0x20,
    FileResume = 0x21,
    FileMetadata = 0x22,
    FileContent = 0x23,
    ApplicationError = 0x24,
    FileListRequest = 0x25,
    FileListResponse = 0x26,
}

#[derive(Debug, PartialEq)]
pub enum AppTlv {
    FileRequest(FileRequest),
    FileResume(FileResume),
    FileMetadata(FileMetadata),
    FileContent(FileContent),
    ApplicationError(ApplicationError),
    FileListRequest(FileListRequest),
    FileListResponse(FileListResponse),
}

impl WireFormat for AppTlv {
    fn write(&self, cursor: &mut Cursor) {
        match self {
            AppTlv::FileRequest(x) => x.write(cursor),
            AppTlv::FileResume(x) => x.write(cursor),
            AppTlv::FileMetadata(x) => x.write(cursor),
            AppTlv::FileContent(x) => x.write(cursor),
            AppTlv::ApplicationError(x) => x.write(cursor),
            AppTlv::FileListRequest(x) => x.write(cursor),
            AppTlv::FileListResponse(x) => x.write(cursor)
        }
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        return match self {
            AppTlv::FileRequest(x) => x.read(cursor),
            AppTlv::FileResume(x) => x.read(cursor),
            AppTlv::FileMetadata(x) => x.read(cursor),
            AppTlv::FileContent(x) => x.read(cursor),
            AppTlv::ApplicationError(x) => x.read(cursor),
            AppTlv::FileListRequest(x) => x.read(cursor),
            AppTlv::FileListResponse(x) => x.read(cursor)
        }
    }
}

/////////////////////////////////
// FileRequest

#[derive(Default, Debug, PartialEq)]
pub struct FileRequest {
    file_paths: Vec<String>,
}

impl WireFormat for FileRequest {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, AppTlvType::FileRequest, {
            write_u8!(cursor, self.file_paths.len() as u8);
            for path in &self.file_paths {
                write_str!(cursor, path);
            }
        })
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, AppTlvType::FileRequest, {
            let mut num_paths = read_u8!(cursor);
            while num_paths > 0 {
                self.file_paths.push(read_str!(cursor));
                num_paths -= 1;
            }
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// FileResume

#[derive(Default, Debug, PartialEq)]
pub struct FileResume {
    file_ids_and_chunk_ids: Vec<(FileId, i64)>,
}

impl WireFormat for FileResume {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, AppTlvType::FileRequest, {
            write_u8!(cursor, self.file_ids_and_chunk_ids.len() as u8);
            for pair in &self.file_ids_and_chunk_ids {
                write_u64!(cursor, pair.0);
                write_i128!(cursor, pair.1);
            }
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, AppTlvType::FileRequest, {
            let mut num_pairs = read_u8!(cursor);
            while num_pairs > 0 {
                self.file_ids_and_chunk_ids.push((
                    read_u64!(cursor),
                    read_i128!(cursor)
                ));
                num_pairs -= 1;
            }
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// FileMetadata

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq)]
#[repr(u8)]
pub enum MetadataEntryType {
    None = 0x00,
    FileName = 0x01,
    FilePath = 0x02,
    FileSize = 0x03,
    NumChunks = 0x04,
    Stat = 0x05,
    Stat64 = 0x06,
    CRC = 0x10,
    MD5 = 0x11,
    SHA0 = 0x12,
    SHA1 = 0x13,
    SHA2 = 0x14,
    SHA3 = 0x15,
}

impl Default for MetadataEntryType {
    fn default() -> MetadataEntryType {MetadataEntryType::None}
}

#[derive(Default, Debug, PartialEq)]
pub struct MetadataEntry {
    code: MetadataEntryType,
    content: Vec<u8>,
}

#[derive(Default, Debug, PartialEq)]
pub struct FileMetadata {
    metadata_entries: Vec<MetadataEntry>,
}

impl WireFormat for FileMetadata {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, AppTlvType::FileMetadata, {
            write_u8!(cursor, self.metadata_entries.len() as u8);
            for entry in &self.metadata_entries {
                write_u8!(cursor, entry.code.to_u8().unwrap());
                write_u8!(cursor, entry.content.len() as u8);
                cursor.write(&entry.content);
            }
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, AppTlvType::FileMetadata, {
            let mut num_entries = read_u8!(cursor);
            while num_entries > 0 {
                self.metadata_entries.push(MetadataEntry{
                    code: match FromPrimitive::from_u8(read_u8!(cursor)) {
                        Some(x) => x,
                        None => MetadataEntryType::None
                    },
                    content: {
                        let mut buf = Vec::new();
                        let mut num_bytes = read_u8!(cursor);
                        while num_bytes > 0 {
                            buf.push(read_u8!(cursor));
                        }
                        buf
                    }
                });
            }
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// FileContent

#[derive(Default, Debug, PartialEq)]
pub struct FileContent {
    content: Vec<u8>,
}

impl WireFormat for FileContent {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, AppTlvType::FileContent, {
            cursor.write(&self.content);
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, AppTlvType::FileContent, {
            cursor.seek(SeekFrom::Current(-2)).expect("Seek back (3) failed.");
            let mut num_bytes = read_u16!(cursor);
            while num_bytes > 0 {
                self.content.push(read_u8!(cursor));
            }
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// ApplicationError

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq)]
#[repr(u8)]
pub enum AppErrorCode {
    None = 0x00,
    FileNotFound = 0x01,
    FileChanged = 0x02,
    NoSpaceLeftOnDisk = 0x03,
    FileHashError = 0x04,
    FileAbort = 0x05,
    InvalidFileResumeRequest = 0x06,
    InvalidDepthForList = 0x07,
    UnknownFormatCode = 0x08,
}

impl Default for AppErrorCode {
    fn default() -> AppErrorCode {AppErrorCode::None}
}

#[derive(Debug, PartialEq)]
pub enum AppErrorData {
    Empty,
    Paths(Vec<String>),
    FormatCodes(Vec<u8>),
}

impl Default for AppErrorData {
    fn default() -> AppErrorData {AppErrorData::Empty}
}

#[derive(Default, Debug, PartialEq)]
pub struct ApplicationError {
    error_code: AppErrorCode,
    error_data: AppErrorData,
}

impl WireFormat for ApplicationError {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, AppTlvType::ApplicationError, {
            write_u8!(cursor, self.error_code.to_u8().unwrap());
            match (&self.error_code, &self.error_data) {
                (AppErrorCode::InvalidDepthForList, AppErrorData::Paths(paths)) => {
                    assert!(paths.len() > 0);
                    write_str!(cursor, paths[0]);
                },
                (AppErrorCode::FileNotFound, AppErrorData::Paths(paths)) |
                (AppErrorCode::FileChanged, AppErrorData::Paths(paths)) |
                (AppErrorCode::FileHashError, AppErrorData::Paths(paths)) |
                (AppErrorCode::FileAbort, AppErrorData::Paths(paths))
                 => {
                    write_u8!(cursor, paths.len() as u8);
                    for path in paths {
                        write_str!(cursor, path);
                    }
                },
                (AppErrorCode::UnknownFormatCode, AppErrorData::FormatCodes(codes)) => {
                    write_u8!(cursor, codes.len() as u8);
                    for code in codes {
                        write_u8!(cursor, *code);
                    }
                },
                (_, _) => {}
            };
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, AppTlvType::ApplicationError, {
            self.error_code = match FromPrimitive::from_u8(read_u8!(cursor)) {
                Some(x) => x,
                None => AppErrorCode::None
            };
            self.error_data = match self.error_code {
                AppErrorCode::InvalidDepthForList => AppErrorData::Paths(
                    vec![read_str!(cursor)]
                ),
                AppErrorCode::FileNotFound |
                AppErrorCode::FileChanged |
                AppErrorCode::FileHashError |
                AppErrorCode::FileAbort
                 => AppErrorData::Paths({
                    let mut num_paths = read_u8!(cursor);
                    let mut paths = Vec::new();
                    while num_paths > 0 {
                        paths.push(read_str!(cursor));
                        num_paths -= 1;
                    }
                    paths
                }),
                AppErrorCode::UnknownFormatCode => AppErrorData::FormatCodes({
                    let mut num_codes = read_u8!(cursor);
                    let mut codes = Vec::new();
                    while num_codes > 0 {
                        codes.push(read_u8!(cursor));
                        num_codes -= 1;
                    }
                    codes
                }),
                _ => AppErrorData::Empty
            }
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// FileListRequest

#[derive(Default, Debug, PartialEq)]
pub struct FileListRequest {
    path: String,
    level_of_recursion: u8,
    format_code: u8,
}

pub const DEFAULT_FORMAT_CODE: u8 = 0x01;

impl WireFormat for FileListRequest {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, AppTlvType::FileListRequest, {
            write_str!(cursor, self.path);
            write_u8!(cursor, self.level_of_recursion);
            write_u8!(cursor, self.format_code);
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, AppTlvType::FileListRequest, {
            self.path = read_str!(cursor);
            self.level_of_recursion = read_u8!(cursor);
            self.format_code = read_u8!(cursor);
        });
        ReadResult::Ok
    }
}

/////////////////////////////////
// FileListResponse

#[derive(Default, Debug, PartialEq)]
pub struct FileListResponse {
    file_list_entries: Vec<FileListEntry>,
}

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq)]
#[repr(u8)]
pub enum FileListEntryType {
    File = 0x00,
    Dir = 0x01
}

impl Default for FileListEntryType {
    fn default() -> Self {FileListEntryType::File}
}

#[derive(Default, Debug, PartialEq)]
pub struct FileListEntry {
    entry_type: FileListEntryType,
    parent: FileId,
    name: String,
    id: FileId
}

impl WireFormat for FileListResponse {
    fn write(&self, cursor: &mut Cursor) {
        write_tlv!(cursor, AppTlvType::FileListResponse, {
            write_u8!(cursor, self.file_list_entries.len() as u8);
            for entry in &self.file_list_entries {
                write_u8!(cursor, entry.entry_type.to_u8().unwrap());
                write_u64!(cursor, entry.parent);
                write_str!(cursor, entry.name);
                write_u64!(cursor, entry.id);
            }
        });
    }

    fn read(&mut self, cursor: &mut Cursor) -> ReadResult {
        read_tlv!(cursor, AppTlvType::FileListResponse, {
            let mut num_entries = read_u8!(cursor);
            while num_entries > 0 {
                self.file_list_entries.push(FileListEntry{
                    entry_type: match FromPrimitive::from_u8(read_u8!(cursor)) {
                        Some(x) => x,
                        None => FileListEntryType::File
                    },
                    parent: read_u64!(cursor),
                    name: read_str!(cursor),
                    id: read_u64!(cursor),
                });
                num_entries -= 1;
            }
        });
        ReadResult::Ok
    }
}

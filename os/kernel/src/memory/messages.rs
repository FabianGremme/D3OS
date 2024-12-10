
use bitflags::bitflags;
use bitfield::*;



#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Oppcode {
RdCurr = 1,
RdOwn = 2,
RdShared = 3,
RdAny = 4,
RdOwnNoData = 5,
ItoMWr = 6,
MemWr = 7,
CLFlush = 8,
CleanEvict = 9,
}

bitfield!{
    struct d2h_request(MSB0[usize]);          //msb0 ist die Bitwertigkeit
    u16;
    valid,_:0, 0;
    oppcode,_:5, 1;
    address,_:51, 6;          // [bool;46],
    cqid,_:63, 52;             //[bool;12],
    nt,_:64, 64;
    rsvd,_:78, 65;                       //[bool;14],
}

bitfield!{
    struct d2h_response(MSB0[usize]);
    u16;
    valid,_:0, 0;
    oppcode,_:5, 1;
    uqid,_:17, 6;                        //[bool;12],
    rsvd,_:19, 18;                       //[bool;14],
}

bitfield!{
    struct d2h_data(MSB0[usize]);
    u16;
    valid,_:0, 0;
    uqid,_:12, 1;
    chunk_valid,_:13, 13;
    bogus,_:14, 14;
    poison,_:15, 15;
    rsvd,_:16, 16;
}


bitfield!{
    struct h2d_request(MSB0[usize]);
    u16;
    valid,_:0, 0;
    opcode,_:3, 1;
    address,_:49, 4;
    uqid,_:61, 50;
    rsvd,_:63, 62;
}
bitfield!{
    struct h2d_response(MSB0[usize]);
    u16;
    valid,_:0, 0;
    opcode,_:4, 1;
    rsp_data,_:16, 5;
    rsp_pre,_:18, 17;
    cqid,_:30, 19;
    rsvd,_:31, 31;
}


bitfield!{
    struct h2d_data(MSB0[usize]);
    u16;
    valid,_:0, 0;
    cqid,_:12, 1;
    chunk_valid,_:13, 13;
    poison,_:14, 14;
    go_err,_:15, 15;
    rsvd,_:23, 16;
}


// das hier sind nun die Nachrichten vom CXL.mem Protokoll:

bitfield!{
    struct m2s_request(MSB0[usize]);
    u16;
    valid,_:0, 0;
    mem_opcode,_:4, 1;
    meta_field,_:6, 5;
    meta_value,_:8, 7;
    snp_type,_:11, 9;
    address,_:58, 12;
    tag,_:74, 59;
    tc,_:76, 75;
    rsvd,_:86, 77;
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum M2S_Req_Oppcode {
    MemInv = 1,
    MemRd = 2,
    MemRdData = 3,
    MemRdFwd = 4,
    MemWrFwd = 5,
    MemInvNT = 6,
    Reserved = 7,
}

bitfield!{
    struct m2s_request_with_data(MSB0[usize]);
    u16;
    valid,_:0, 0;
    mem_opcode,_:4, 1;
    meta_field,_:6, 5;
    meta_value,_:8, 7;
    snp_type,_:11, 9;
    address,_:57, 12;
    tag,_:73, 58;
    tc,_:75, 74;
    poison,_:76, 76;
    rsvd,_:86, 77;
}

bitfield!{
    struct s2m_no_data_response(MSB0[usize]);
    u16;
    valid,_:0, 0;
    opcode,_:3, 1;
    meta_field,_:5, 4;
    meta_value,_:7, 6;
    tag,_:23, 8;
    rsvd,_:27, 24;
}

bitfield!{
    struct s2m_data_response(MSB0[usize]);
    u16;
    valid,_:0, 0;
    opcode,_:3, 1;
    meta_field,_:5, 4;
    meta_value,_:7, 6;
    tag,_:23, 8;
    poison,_:24, 24;
    rsvd,_:39, 25;
}
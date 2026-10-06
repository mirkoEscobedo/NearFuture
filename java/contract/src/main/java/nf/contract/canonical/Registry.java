package nf.contract.canonical;

final class Registry {
    private Registry() { }
    enum Kind { ID, DIGEST, U8, U32, U64, BOOL, TEXT, BYTES, OPTIONAL64, REVISIONS, SET, NESTED, DOCUMENTS, OUTCOME }
    record Field(Kind kind, int domain, int type, int sortField) { }
    static Field scalar(Kind kind) { return new Field(kind, 0, 0, -1); }
    static Field nested(int domain, int type) { return new Field(Kind.NESTED, domain, type, -1); }
    static Field documents(int type, int sortField) { return new Field(Kind.DOCUMENTS, 6, type, sortField); }
    private static final Field ID = scalar(Kind.ID), DIGEST = scalar(Kind.DIGEST), U32 = scalar(Kind.U32),
            U64 = scalar(Kind.U64), REVISIONS = scalar(Kind.REVISIONS), SET = scalar(Kind.SET),
            OPT = scalar(Kind.OPTIONAL64), PAYLOAD = nested(6, 1), REQUIRED = nested(6, 2);
    static Field[] fields(int domain, int type) {
        if (domain < 0 || domain > 65535 || type < 0 || type > 65535) throw new Validation(Validation.Code.UNKNOWN_RECORD);
        return switch ((domain << 16) | type) {
            case 65537 -> new Field[] {ID, ID, ID, ID, ID, U32, DIGEST};
            case 65538 -> new Field[] {nested(1, 1), REVISIONS, PAYLOAD, OPT, REQUIRED};
            case 131073 -> new Field[] {ID, ID, U64, U64, DIGEST, REVISIONS, documents(5, 0), documents(4, 0),
                    documents(6, 0), documents(7, 0), documents(8, 1), documents(9, 0), REQUIRED};
            case 196609 -> new Field[] {ID, ID, U32, DIGEST, DIGEST, U64, REVISIONS, REVISIONS,
                    documents(1, -1), documents(4, 0), REQUIRED};
            case 262145 -> new Field[] {ID, U64, DIGEST, U64, U64, DIGEST, documents(8, 1), documents(1, -1),
                    documents(4, 0), DIGEST, REQUIRED};
            case 393217 -> new Field[] {U32, scalar(Kind.BYTES)};
            case 393218 -> new Field[] {SET, SET};
            case 393219 -> new Field[] {ID, U64};
            case 393220 -> new Field[] {ID, U32, PAYLOAD};
            case 393221 -> new Field[] {ID, nested(6, 3), PAYLOAD};
            case 393222 -> new Field[] {ID, U64, PAYLOAD};
            case 393223 -> new Field[] {ID, PAYLOAD};
            case 393224 -> new Field[] {ID, ID, ID, DIGEST, U32, scalar(Kind.OUTCOME), OPT};
            case 393225 -> new Field[] {ID, DIGEST, nested(6, 8)};
            case 393226 -> new Field[] {U32, scalar(Kind.TEXT), SET, SET, scalar(Kind.BOOL)};
            default -> throw new Validation(Validation.Code.UNKNOWN_RECORD);
        };
    }
}

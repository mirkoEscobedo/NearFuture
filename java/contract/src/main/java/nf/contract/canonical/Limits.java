package nf.contract.canonical;

/** Negotiated values may lower, never raise, these implementation hard caps. */
public record Limits(int documentBytes, int textBytes, int bytesBytes, int collectionItems,
                     int totalItems, int nestingDepth, int allocationBytes) {
    public static final Limits DEFAULT = new Limits(1_048_576, 4096, 262_144, 4096, 16_384, 32, 4_194_304);
    public Limits {
        if (documentBytes < 1 || documentBytes > 1_048_576 || textBytes < 1 || textBytes > 4096
                || bytesBytes < 1 || bytesBytes > 262_144 || collectionItems < 1 || collectionItems > 4096
                || totalItems < 1 || totalItems > 16_384 || nestingDepth < 1 || nestingDepth > 32
                || allocationBytes < 1 || allocationBytes > 4_194_304) {
            throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        }
    }
}

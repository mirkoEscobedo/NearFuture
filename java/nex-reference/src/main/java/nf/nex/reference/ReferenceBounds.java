package nf.nex.reference;

import java.util.Collection;

final class ReferenceBounds {
    private ReferenceBounds() {}
    static void finite(float... values) {
        for (float value : values) if (!Float.isFinite(value)) throw new IllegalArgumentException("Nonfinite reference fact");
    }
    static void count(Collection<?> values, int maximum) {
        if (values == null || values.size() > maximum) throw new IllegalArgumentException("Missing or oversized reference facts");
    }
}

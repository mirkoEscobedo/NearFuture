package nf.contract.system;

import java.util.Arrays;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPlacementRegistry.ConceptualSlot;
import nf.contract.system.SystemPlacementRegistry.PlacementEntry;

/**
 * Pure Grid256V1 coordinates and source metadata, not permission to render or import.
 * Registry access requires its existing confinement or external serialization.
 */
public final class SystemPresentation {
    private static final long GRID = 256;
    private static final long MAX_ORIGIN = 1L << 40;
    private static final int MAX_DIMENSION = 1 << 24;
    private static final int MAX_HALF_EXTENT = 32767;
    private final Id universe;
    private final Domain domain;

    public enum Profile { GRID_256_V1 }

    /** Ordinary values; the enclosing presentation constructor checks bounds and fit. */
    public record Domain(long originX, long originY, int width, int height, int halfExtent) { }

    public enum ErrorCode {
        INVALID_NAMESPACE,
        BOUNDS,
        DOES_NOT_FIT,
        INVALID_INPUT,
        UNIVERSE_MISMATCH,
        MISSING_PLACEMENT
    }

    public static final class Failure extends IllegalArgumentException {
        private static final long serialVersionUID = 1L;
        private final ErrorCode code;

        private Failure(ErrorCode code) {
            super(code.name(), null);
            this.code = code;
        }

        public ErrorCode code() { return code; }
    }

    public static final class ProjectedPlacement {
        private final Id universe;
        private final Id history;
        private final Id owner;
        private final Id system;
        private final ConceptualSlot slot;
        private final long revision;
        private final byte[] topologySha256;
        private final Domain domain;
        private final long x;
        private final long y;

        private ProjectedPlacement(Id universe, PlacementEntry entry, Domain domain, long x, long y) {
            this.universe = copy(universe);
            this.history = copy(entry.history());
            this.owner = copy(entry.owner());
            this.system = copy(entry.system());
            this.slot = entry.slot();
            this.revision = entry.revision();
            this.topologySha256 = entry.topologySha256().clone();
            this.domain = domain;
            this.x = x;
            this.y = y;
        }

        public Profile profile() { return Profile.GRID_256_V1; }
        public Id universe() { return copy(universe); }
        public Id history() { return copy(history); }
        public Id owner() { return copy(owner); }
        public Id system() { return copy(system); }
        public ConceptualSlot slot() { return slot; }
        public long revision() { return revision; }
        public byte[] topologySha256() { return topologySha256.clone(); }
        public Domain domain() { return domain; }
        public long x() { return x; }
        public long y() { return y; }

        @Override
        public boolean equals(Object other) {
            if (this == other) return true;
            if (!(other instanceof ProjectedPlacement value)) return false;
            return profile() == value.profile()
                    && universe.equals(value.universe)
                    && history.equals(value.history)
                    && owner.equals(value.owner)
                    && system.equals(value.system)
                    && slot.equals(value.slot)
                    && revision == value.revision
                    && Arrays.equals(topologySha256, value.topologySha256)
                    && domain.equals(value.domain)
                    && x == value.x
                    && y == value.y;
        }

        @Override
        public int hashCode() {
            int result = profile().hashCode();
            result = 31 * result + universe.hashCode();
            result = 31 * result + history.hashCode();
            result = 31 * result + owner.hashCode();
            result = 31 * result + system.hashCode();
            result = 31 * result + slot.hashCode();
            result = 31 * result + Long.hashCode(revision);
            result = 31 * result + Arrays.hashCode(topologySha256);
            result = 31 * result + domain.hashCode();
            result = 31 * result + Long.hashCode(x);
            return 31 * result + Long.hashCode(y);
        }

        @Override
        public String toString() { return "ProjectedPlacement{profile=GRID_256_V1}"; }
    }

    public SystemPresentation(Id universe, Domain domain) {
        if (invalid(universe)) throw fail(ErrorCode.INVALID_NAMESPACE);
        if (domain == null || !bounded(domain)) throw fail(ErrorCode.BOUNDS);
        long minimumPitch;
        try {
            minimumPitch = Math.addExact(Math.multiplyExact(2L, domain.halfExtent()), 1L);
        } catch (ArithmeticException error) {
            throw fail(ErrorCode.BOUNDS);
        }
        if (domain.width() / GRID < minimumPitch || domain.height() / GRID < minimumPitch) {
            throw fail(ErrorCode.DOES_NOT_FIT);
        }
        this.universe = copy(universe);
        this.domain = domain;
    }

    public Profile profile() { return Profile.GRID_256_V1; }
    public Id universe() { return copy(universe); }
    public Domain domain() { return domain; }

    public ProjectedPlacement project(SystemPlacementRegistry registry, Id history, Id owner, Id system) {
        if (registry == null || invalid(history) || invalid(owner) || invalid(system)) {
            throw fail(ErrorCode.INVALID_INPUT);
        }
        if (!universe.equals(registry.universe())) throw fail(ErrorCode.UNIVERSE_MISMATCH);
        for (PlacementEntry entry : registry.entries()) {
            if (entry.history().equals(history) && entry.owner().equals(owner)
                    && entry.system().equals(system)) {
                long slot = entry.slot().value();
                long x;
                long y;
                try {
                    x = coordinate(domain.originX(), slot % GRID, domain.width() / GRID);
                    y = coordinate(domain.originY(), slot / GRID, domain.height() / GRID);
                } catch (ArithmeticException error) {
                    throw fail(ErrorCode.BOUNDS);
                }
                return new ProjectedPlacement(universe, entry, domain, x, y);
            }
        }
        throw fail(ErrorCode.MISSING_PLACEMENT);
    }

    @Override
    public String toString() { return "SystemPresentation{profile=GRID_256_V1}"; }

    private static boolean bounded(Domain value) {
        return value.originX() >= -MAX_ORIGIN && value.originX() <= MAX_ORIGIN
                && value.originY() >= -MAX_ORIGIN && value.originY() <= MAX_ORIGIN
                && value.width() >= 1 && value.width() <= MAX_DIMENSION
                && value.height() >= 1 && value.height() <= MAX_DIMENSION
                && value.halfExtent() >= 0 && value.halfExtent() <= MAX_HALF_EXTENT;
    }

    private static long coordinate(long origin, long index, long pitch) {
        // Bounds establish index <= 255, pitch <= 65536 and |coordinate| < 2^41.
        // Only nonnegative pitch/index are divided; right/top remainders stay unused.
        long offset = Math.addExact(Math.multiplyExact(index, pitch), pitch / 2);
        return Math.addExact(origin, offset);
    }

    private static Id copy(Id value) { return new Id(value.value()); }

    private static boolean invalid(Id value) {
        if (value == null) return true;
        for (byte part : value.value()) if (part != 0) return false;
        return true;
    }

    private static Failure fail(ErrorCode code) { return new Failure(code); }
}

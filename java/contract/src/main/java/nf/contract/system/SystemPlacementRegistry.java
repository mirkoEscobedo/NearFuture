package nf.contract.system;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPackageTopology.Limits;
import nf.contract.system.SystemPackageTopology.Topology;

/**
 * Bounded current conceptual assignments, not publication or native occupancy authority.
 * Confine each registry to one thread or externally serialize all access.
 */
public final class SystemPlacementRegistry {
    private static final int MAX_ROWS = 256;
    private final Id universe;
    private final int capacity;
    private final List<PlacementEntry> entries;

    public record ConceptualSlot(int value) {
        public ConceptualSlot {
            if (value < 0 || value > 65535) throw fail(PlacementError.BOUNDS);
        }
    }

    public enum ReservationOutcome { INSERTED, UNCHANGED, UPDATED }

    public enum PlacementError {
        INVALID_NAMESPACE,
        BOUNDS,
        UNIVERSE_MISMATCH,
        SLOT_OCCUPIED,
        SLOT_MISMATCH,
        STALE_REVISION,
        REVISION_CONFLICT,
        TOPOLOGY
    }

    public static final class Failure extends IllegalArgumentException {
        private static final long serialVersionUID = 1L;
        private final PlacementError code;
        private final SystemPackageTopology.Code topologyCode;

        private Failure(PlacementError code, SystemPackageTopology.Code topologyCode) {
            super(code.name(), null);
            this.code = code;
            this.topologyCode = topologyCode;
        }

        public PlacementError code() { return code; }
        public SystemPackageTopology.Code topologyCode() { return topologyCode; }
    }

    public static final class PlacementEntry {
        private final Id history;
        private final Id owner;
        private final Id system;
        private final ConceptualSlot slot;
        private final long revision;
        private final byte[] topologySha256;

        private PlacementEntry(Id history, Id owner, Id system, ConceptualSlot slot,
                long revision, byte[] topologySha256) {
            this.history = copy(history);
            this.owner = copy(owner);
            this.system = copy(system);
            this.slot = slot;
            this.revision = revision;
            this.topologySha256 = topologySha256.clone();
        }

        public Id history() { return copy(history); }
        public Id owner() { return copy(owner); }
        public Id system() { return copy(system); }
        public ConceptualSlot slot() { return slot; }
        public long revision() { return revision; }
        public byte[] topologySha256() { return topologySha256.clone(); }

        @Override
        public boolean equals(Object other) {
            if (this == other) return true;
            if (!(other instanceof PlacementEntry entry)) return false;
            return history.equals(entry.history)
                    && owner.equals(entry.owner)
                    && system.equals(entry.system)
                    && slot.equals(entry.slot)
                    && revision == entry.revision
                    && Arrays.equals(topologySha256, entry.topologySha256);
        }

        @Override
        public int hashCode() {
            int result = history.hashCode();
            result = 31 * result + owner.hashCode();
            result = 31 * result + system.hashCode();
            result = 31 * result + slot.hashCode();
            result = 31 * result + Long.hashCode(revision);
            return 31 * result + Arrays.hashCode(topologySha256);
        }
    }

    public SystemPlacementRegistry(Id universe, int capacity) {
        if (universe == null || zero(universe)) throw fail(PlacementError.INVALID_NAMESPACE);
        if (capacity < 1 || capacity > MAX_ROWS) throw fail(PlacementError.BOUNDS);
        this.universe = copy(universe);
        this.capacity = capacity;
        this.entries = new ArrayList<>(capacity);
    }

    public Id universe() { return copy(universe); }
    public int capacity() { return capacity; }

    /** Immutable rows are shared with a new bounded list snapshot, never a live view. */
    public List<PlacementEntry> entries() { return List.copyOf(entries); }

    public ReservationOutcome reserve(Topology topology, ConceptualSlot slot, Limits limits) {
        try {
            SystemPackageTopology.validate(topology, limits);
        } catch (SystemPackageTopology.Failure error) {
            throw topologyFailure(error);
        }
        if (!topology.universe().equals(universe)) throw fail(PlacementError.UNIVERSE_MISMATCH);
        byte[] topologySha256;
        try {
            topologySha256 = SystemPackageTopology.digest(topology, limits);
        } catch (SystemPackageTopology.Failure error) {
            throw topologyFailure(error);
        }
        // Java-only null refusal follows the unchanged validation/namespace/digest prelude.
        if (slot == null) throw fail(PlacementError.BOUNDS);

        int index = insertionIndex(topology);
        if (index < entries.size() && compareKey(entries.get(index), topology) == 0) {
            PlacementEntry current = entries.get(index);
            if (!slot.equals(current.slot)) throw fail(PlacementError.SLOT_MISMATCH);
            int comparison = Long.compareUnsigned(topology.revision(), current.revision);
            if (comparison < 0) throw fail(PlacementError.STALE_REVISION);
            if (comparison == 0) {
                if (!Arrays.equals(topologySha256, current.topologySha256)) {
                    throw fail(PlacementError.REVISION_CONFLICT);
                }
                return ReservationOutcome.UNCHANGED;
            }
            PlacementEntry replacement = new PlacementEntry(current.history, current.owner,
                    current.system, current.slot, topology.revision(), topologySha256);
            entries.set(index, replacement);
            return ReservationOutcome.UPDATED;
        }

        for (PlacementEntry current : entries) {
            if (slot.equals(current.slot)) throw fail(PlacementError.SLOT_OCCUPIED);
        }
        if (entries.size() == capacity) throw fail(PlacementError.BOUNDS);
        PlacementEntry inserted = new PlacementEntry(topology.history(), topology.owner(),
                topology.system(), slot, topology.revision(), topologySha256);
        entries.add(index, inserted);
        return ReservationOutcome.INSERTED;
    }

    private int insertionIndex(Topology topology) {
        int low = 0;
        int high = entries.size();
        while (low < high) {
            int middle = (low + high) >>> 1;
            if (compareKey(entries.get(middle), topology) < 0) low = middle + 1;
            else high = middle;
        }
        return low;
    }

    private static int compareKey(PlacementEntry entry, Topology topology) {
        int comparison = Arrays.compareUnsigned(entry.history.value(), topology.history().value());
        if (comparison != 0) return comparison;
        comparison = Arrays.compareUnsigned(entry.owner.value(), topology.owner().value());
        if (comparison != 0) return comparison;
        return Arrays.compareUnsigned(entry.system.value(), topology.system().value());
    }

    private static Id copy(Id value) { return new Id(value.value()); }

    private static boolean zero(Id value) {
        for (byte part : value.value()) if (part != 0) return false;
        return true;
    }

    private static Failure fail(PlacementError code) { return new Failure(code, null); }

    private static Failure topologyFailure(SystemPackageTopology.Failure error) {
        return new Failure(PlacementError.TOPOLOGY, error.code());
    }
}

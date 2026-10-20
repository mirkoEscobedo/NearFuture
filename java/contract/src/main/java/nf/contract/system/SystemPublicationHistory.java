package nf.contract.system;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPackageTopology.Topology;

/**
 * Bounded owned data history, not owner, storage, publication or game authority.
 * Confine each mutable history to one thread or externally serialize all access.
 */
public final class SystemPublicationHistory {
    private static final int MAX_KEYS = 256;
    private static final int MAX_RECORDS = 512;
    private static final int MAX_CANONICAL_BYTES = 8_388_608;

    public enum Profile { SELECTED_TOPOLOGY_V1 }
    public enum Kind { PUBLISHED_VERSION, TOMBSTONE }
    public record Limits(int keys, int records, int canonicalBytes) { }
    public enum Outcome { PUBLICATION_APPENDED, TOMBSTONE_APPENDED, UNCHANGED }
    public enum PublicationError {
        INVALID_NAMESPACE,
        BOUNDS,
        INVALID_INPUT,
        UNIVERSE_MISMATCH,
        MISSING_PUBLICATION,
        STALE_REVISION,
        REVISION_CONFLICT,
        KEY_LIMIT,
        RECORD_LIMIT,
        BYTE_LIMIT,
        TOPOLOGY
    }

    public static final class Failure extends IllegalArgumentException {
        private static final long serialVersionUID = 1L;
        private final PublicationError code;
        private final SystemPackageTopology.Code topologyCode;

        private Failure(PublicationError code, SystemPackageTopology.Code topologyCode) {
            super(code.name(), null);
            this.code = code;
            this.topologyCode = topologyCode;
        }

        public PublicationError code() { return code; }
        public SystemPackageTopology.Code topologyCode() { return topologyCode; }
    }

    public static final class PublicationRecord {
        private final Id universe;
        private final Id history;
        private final Id owner;
        private final Id system;
        private final Profile profile;
        private final Kind kind;
        private final long revision;
        private final long referencedPublicationRevision;
        private final byte[] referencedTopologySha256;
        private final byte[] canonicalTopologyBytes;

        /** The optional body is fresh private codec output, transferred without another body copy. */
        private PublicationRecord(Id universe, Id history, Id owner, Id system, Kind kind,
                long revision, long referencedPublicationRevision, byte[] hash, byte[] ownedBody) {
            this.universe = copy(universe);
            this.history = copy(history);
            this.owner = copy(owner);
            this.system = copy(system);
            this.profile = Profile.SELECTED_TOPOLOGY_V1;
            this.kind = kind;
            this.revision = revision;
            this.referencedPublicationRevision = referencedPublicationRevision;
            this.referencedTopologySha256 = hash.clone();
            this.canonicalTopologyBytes = ownedBody;
        }

        public Id universe() { return copy(universe); }
        public Id history() { return copy(history); }
        public Id owner() { return copy(owner); }
        public Id system() { return copy(system); }
        public Profile profile() { return profile; }
        public Kind kind() { return kind; }
        public long revision() { return revision; }
        public long referencedPublicationRevision() { return referencedPublicationRevision; }
        public byte[] referencedTopologySha256() { return referencedTopologySha256.clone(); }
        public byte[] canonicalTopologyBytes() {
            return canonicalTopologyBytes == null ? null : canonicalTopologyBytes.clone();
        }

        @Override
        public boolean equals(Object other) {
            if (this == other) return true;
            if (!(other instanceof PublicationRecord record)) return false;
            return universe.equals(record.universe)
                    && history.equals(record.history)
                    && owner.equals(record.owner)
                    && system.equals(record.system)
                    && profile == record.profile
                    && kind == record.kind
                    && revision == record.revision
                    && referencedPublicationRevision == record.referencedPublicationRevision
                    && Arrays.equals(referencedTopologySha256, record.referencedTopologySha256)
                    && Arrays.equals(canonicalTopologyBytes, record.canonicalTopologyBytes);
        }

        @Override
        public int hashCode() {
            int result = universe.hashCode();
            result = 31 * result + history.hashCode();
            result = 31 * result + owner.hashCode();
            result = 31 * result + system.hashCode();
            result = 31 * result + profile.hashCode();
            result = 31 * result + kind.hashCode();
            result = 31 * result + Long.hashCode(revision);
            result = 31 * result + Long.hashCode(referencedPublicationRevision);
            result = 31 * result + Arrays.hashCode(referencedTopologySha256);
            return 31 * result + Arrays.hashCode(canonicalTopologyBytes);
        }

        @Override
        public String toString() {
            return "PublicationRecord[profile=" + profile + ", kind=" + kind
                    + ", revision=" + Long.toUnsignedString(revision)
                    + ", referencedPublicationRevision=" + Long.toUnsignedString(referencedPublicationRevision)
                    + ", canonicalBytes=" + (canonicalTopologyBytes == null ? 0 : canonicalTopologyBytes.length) + "]";
        }
    }

    private final Id universe;
    private final Limits limits;
    private final List<PublicationRecord> records;
    private int keys;
    private int canonicalBytes;

    public SystemPublicationHistory(Id universe, Limits limits) {
        if (universe == null || zero(universe)) throw fail(PublicationError.INVALID_NAMESPACE);
        if (limits == null || limits.keys() < 1 || limits.keys() > MAX_KEYS
                || limits.records() < 1 || limits.records() > MAX_RECORDS
                || limits.canonicalBytes() < 1 || limits.canonicalBytes() > MAX_CANONICAL_BYTES) {
            throw fail(PublicationError.BOUNDS);
        }
        this.universe = copy(universe);
        this.limits = limits;
        this.records = new ArrayList<>();
        this.keys = 0;
        this.canonicalBytes = 0;
    }

    public Id universe() { return copy(universe); }
    public Limits limits() { return limits; }
    public int keysLen() { return keys; }
    public int canonicalBytesLen() { return canonicalBytes; }
    public List<PublicationRecord> records() { return List.copyOf(records); }

    public PublicationRecord latest(Id history, Id owner, Id system) {
        if (history == null || owner == null || system == null
                || zero(history) || zero(owner) || zero(system)) return null;
        for (int i = records.size() - 1; i >= 0; i--) {
            PublicationRecord record = records.get(i);
            if (compareKey(record, history, owner, system) == 0) return record;
        }
        return null;
    }

    public Outcome appendPublication(Topology topology, SystemPackageTopology.Limits topologyLimits) {
        try {
            SystemPackageTopology.validate(topology, topologyLimits);
        } catch (SystemPackageTopology.Failure error) {
            throw topologyFailure(error);
        }
        if (!topology.universe().equals(universe)) throw fail(PublicationError.UNIVERSE_MISMATCH);
        byte[] body;
        byte[] hash;
        try {
            body = SystemPackageTopology.encode(topology, topologyLimits);
            hash = SystemPackageTopology.digest(topology, topologyLimits);
        } catch (SystemPackageTopology.Failure error) {
            throw topologyFailure(error);
        }
        int index = position(topology.history(), topology.owner(), topology.system(), topology.revision());
        if (index < records.size()) {
            PublicationRecord existing = records.get(index);
            if (compareKey(existing, topology.history(), topology.owner(), topology.system()) == 0
                    && existing.revision == topology.revision()) {
                if (existing.kind == Kind.PUBLISHED_VERSION
                        && existing.profile == Profile.SELECTED_TOPOLOGY_V1
                        && Arrays.equals(existing.canonicalTopologyBytes, body)) return Outcome.UNCHANGED;
                throw fail(PublicationError.REVISION_CONFLICT);
            }
        }
        PublicationRecord latest = latest(topology.history(), topology.owner(), topology.system());
        if (latest != null && Long.compareUnsigned(topology.revision(), latest.revision) < 0) {
            throw fail(PublicationError.STALE_REVISION);
        }
        int[] next = preflight(latest == null, body.length);
        PublicationRecord record = new PublicationRecord(universe, topology.history(), topology.owner(), topology.system(),
                Kind.PUBLISHED_VERSION, topology.revision(), topology.revision(), hash, body);
        records.add(index, record);
        keys = next[0];
        canonicalBytes = next[1];
        return Outcome.PUBLICATION_APPENDED;
    }

    public Outcome appendTombstone(Id history, Id owner, Id system, long revision) {
        if (history == null || owner == null || system == null
                || zero(history) || zero(owner) || zero(system) || revision == 0) {
            throw fail(PublicationError.INVALID_INPUT);
        }
        PublicationRecord latest = latest(history, owner, system);
        if (latest == null) throw fail(PublicationError.MISSING_PUBLICATION);
        int index = position(history, owner, system, revision);
        if (index < records.size()) {
            PublicationRecord existing = records.get(index);
            if (compareKey(existing, history, owner, system) == 0 && existing.revision == revision) {
                if (existing.kind == Kind.TOMBSTONE) return Outcome.UNCHANGED;
                throw fail(PublicationError.REVISION_CONFLICT);
            }
        }
        if (Long.compareUnsigned(revision, latest.revision) < 0) throw fail(PublicationError.STALE_REVISION);
        PublicationRecord publication = null;
        for (int i = records.size() - 1; i >= 0; i--) {
            PublicationRecord record = records.get(i);
            if (compareKey(record, history, owner, system) == 0 && record.kind == Kind.PUBLISHED_VERSION) {
                publication = record;
                break;
            }
        }
        if (publication == null) throw fail(PublicationError.MISSING_PUBLICATION);
        int[] next = preflight(false, 0);
        PublicationRecord record = new PublicationRecord(universe, history, owner, system, Kind.TOMBSTONE,
                revision, publication.revision, publication.referencedTopologySha256, null);
        records.add(index, record);
        keys = next[0];
        canonicalBytes = next[1];
        return Outcome.TOMBSTONE_APPENDED;
    }

    private int position(Id history, Id owner, Id system, long revision) {
        int low = 0, high = records.size();
        while (low < high) {
            int middle = (low + high) >>> 1;
            PublicationRecord record = records.get(middle);
            int comparison = compareKey(record, history, owner, system);
            if (comparison == 0) comparison = Long.compareUnsigned(record.revision, revision);
            if (comparison < 0) low = middle + 1;
            else high = middle;
        }
        return low;
    }

    private int[] preflight(boolean newKey, int bodyBytes) {
        int nextKeys = checkedAdd(keys, newKey ? 1 : 0, limits.keys(), PublicationError.KEY_LIMIT);
        checkedAdd(records.size(), 1, limits.records(), PublicationError.RECORD_LIMIT);
        int nextBytes = checkedAdd(canonicalBytes, bodyBytes, limits.canonicalBytes(), PublicationError.BYTE_LIMIT);
        return new int[] {nextKeys, nextBytes};
    }

    private static int checkedAdd(int current, int increment, int limit, PublicationError error) {
        if (current < 0 || increment < 0 || current > limit || increment > limit - current) throw fail(error);
        return current + increment;
    }

    private static int compareKey(PublicationRecord record, Id history, Id owner, Id system) {
        int comparison = Arrays.compareUnsigned(record.history.value(), history.value());
        if (comparison != 0) return comparison;
        comparison = Arrays.compareUnsigned(record.owner.value(), owner.value());
        if (comparison != 0) return comparison;
        return Arrays.compareUnsigned(record.system.value(), system.value());
    }

    private static boolean zero(Id id) {
        for (byte value : id.value()) if (value != 0) return false;
        return true;
    }

    private static Id copy(Id id) { return new Id(id.value()); }
    private static Failure fail(PublicationError code) { return new Failure(code, null); }
    private static Failure topologyFailure(SystemPackageTopology.Failure error) {
        return new Failure(PublicationError.TOPOLOGY, error.code());
    }

    @Override
    public String toString() {
        return "SystemPublicationHistory[limits=" + limits + ", keys=" + keys
                + ", records=" + records.size() + ", canonicalBytes=" + canonicalBytes + "]";
    }
}

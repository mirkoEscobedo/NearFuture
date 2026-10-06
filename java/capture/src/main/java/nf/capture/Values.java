package nf.capture;
import java.util.Arrays;
import java.util.List;
import java.util.Objects;
/** Private in-process values. No wire registration or native mutation authority is implied. */
public final class Values {
    private Values() { }
    public static final int MAX_ITEMS=64, MAX_BYTES=8192, MAX_PER_FRAME=8;
    public static final class Id implements Comparable<Id> {
        private final byte[] bytes;
        public Id(byte[] bytes) { if (bytes==null || bytes.length!=16) fail(); this.bytes=bytes.clone(); }
        public byte[] bytes() { return bytes.clone(); }
        @Override public int compareTo(Id other) { return Arrays.compareUnsigned(bytes,other.bytes); }
        @Override public boolean equals(Object other) {return other instanceof Id id && Arrays.equals(bytes,id.bytes);}
        @Override public int hashCode() {return Arrays.hashCode(bytes);}
    }
    public static final class Hash {
        private final byte[] bytes;
        public Hash(byte[] bytes) { if (bytes==null || bytes.length!=32) fail(); this.bytes=bytes.clone(); }
        public byte[] bytes() { return bytes.clone(); }
        @Override public boolean equals(Object other) {return other instanceof Hash hash && Arrays.equals(bytes,hash.bytes);}
        @Override public int hashCode() {return Arrays.hashCode(bytes);}
    }
    public record Context(Id universe,Id history,Id campaign,Id branch,Id provider,long session,Hash content,Hash ruleset,long ownershipGeneration) {
        public Context { if (universe==null||history==null||campaign==null||branch==null||provider==null||content==null||ruleset==null||session==0||ownershipGeneration<0) fail(); }
    }
    public record Revision(Id aggregate,long revision) {
        public Revision { if (aggregate==null||revision<0) fail(); }
    }
    public record SourceStamp(Context context,long generation,List<Revision> readSet) {
        public SourceStamp {
            Objects.requireNonNull(context); if (generation<0) fail(); bounded(readSet.size());
            readSet=List.copyOf(readSet); Id previous=null;
            for (Revision value:readSet) {if (previous!=null && previous.compareTo(value.aggregate())>=0) fail(); previous=value.aggregate();}
        }
    }
    public sealed interface Input permits Faction,Relation,Config {
        Id aggregate(); long revision(); int bytes();
    }
    public record Faction(Id aggregate,long revision,int wearinessBits) implements Input {
        public Faction { field(aggregate,revision); float value=finite(wearinessBits); if (value<0) fail(); }
        public int bytes() {return 29;}
    }
    public record Relation(Id aggregate,long revision,Id from,Id to,int relationBits) implements Input {
        public Relation {field(aggregate,revision); if (from==null||to==null||from.equals(to)) fail(); float value=finite(relationBits); if (value< -1 || value>1) fail();}
        public int bytes() {return 61;}
    }
    public record Config(Id aggregate,long revision,int minimumPeaceWearinessBits) implements Input {
        public Config {field(aggregate,revision); if (finite(minimumPeaceWearinessBits)<=0) fail();}
        public int bytes() {return 29;}
    }
    public record Snapshot(SourceStamp certificate,List<Input> inputs) {
        public Snapshot {
            Objects.requireNonNull(certificate); bounded(inputs.size()); inputs=List.copyOf(inputs);
            if (inputs.size()!=certificate.readSet().size()) fail(); int bytes=256+24*inputs.size();
            for (int n=0;n<inputs.size();n++) { Input input=inputs.get(n); Revision expected=certificate.readSet().get(n);
                if (!input.aggregate().equals(expected.aggregate()) || input.revision()!=expected.revision()) fail(); bytes+=input.bytes(); }
            if (bytes>MAX_BYTES) throw new CaptureFailure(CaptureFailure.Code.LIMIT);
        }
        public Eligibility eligibility() {return Eligibility.SHADOW_ONLY;}
    }
    public enum Eligibility { SHADOW_ONLY }
    static void field(Id aggregate,long revision) {if (aggregate==null||revision<0) fail();}
    static float finite(int bits) {float value=Float.intBitsToFloat(bits); if (!Float.isFinite(value)) fail(); return value;}
    static void bounded(int size) {if (size<1||size>MAX_ITEMS) throw new CaptureFailure(CaptureFailure.Code.LIMIT);}
    static void fail() {throw new CaptureFailure(CaptureFailure.Code.INVALID);}
}

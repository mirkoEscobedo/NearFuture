package nf.capture;
import java.util.List;
import static nf.capture.Values.*;
public final class CaptureSmoke {
    private CaptureSmoke() { }
    public static void main(String[] arguments) throws InterruptedException {
        CaptureChurnTest.run();
        DirtyIndexTest.run();
        DirtyOverflowTest.run();
        ProjectionTest.run();
        LifecycleTest.run();
        SourceFaultTest.run();
        ValuesBoundaryTest.run();
        SyntheticSource source = new SyntheticSource();
        CampaignCapture capture = new CampaignCapture(source, Thread.currentThread(), 1);
        if (!capture.begin(0)) throw new AssertionError("capture starts");
        if (capture.advance(0) != CampaignCapture.Result.PARTIAL) throw new AssertionError("one bounded chunk");
        if (capture.advance(0) != CampaignCapture.Result.PARTIAL || source.reads != 1)
            throw new AssertionError("repeated callback in the same frame cannot exceed work budget");
        source.generation++;
        if (capture.advance(1) != CampaignCapture.Result.STALE || capture.published() != null)
            throw new AssertionError("mixed-generation immutable values must never publish");
        System.out.println("PASS: eight bounded capture/projection behavior groups; native G1 remains unobserved");
    }
    static final class SyntheticSource implements CaptureSource {
        long generation; int reads; boolean failRead; int mutateAt;
        Context context = new Context(id(1),id(2),id(3),id(4),id(5),1,hash(6),hash(7),1);
        List<Input> values = List.of(new Faction(id(10),0,Float.floatToRawIntBits(75)),
            new Config(id(11),0,Float.floatToRawIntBits(100)));
        public SourceStamp stamp() {
            return new SourceStamp(context,generation,values.stream().map(value -> new Revision(value.aggregate(),value.revision())).toList());
        }
        public Input read(Id aggregate) { reads++; if (failRead) throw new IllegalStateException("private source details"); if (reads==mutateAt) generation++;
            return values.stream().filter(value -> value.aggregate().equals(aggregate)).findFirst().orElseThrow();
        }
    }
    static Id id(int value) {byte[] bytes=new byte[16];bytes[15]=(byte)value;return new Id(bytes);}
    static Hash hash(int value) {byte[] bytes=new byte[32];bytes[31]=(byte)value;return new Hash(bytes);}
}

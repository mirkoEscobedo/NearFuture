package nf.capture;
final class SourceFaultTest {
    private SourceFaultTest() { }
    static void run() {
        CaptureSmoke.SyntheticSource source=new CaptureSmoke.SyntheticSource();
        CampaignCapture capture=new CampaignCapture(source,Thread.currentThread(),1);
        capture.begin(0);capture.advance(0);source.failRead=true;
        LifecycleTest.expect(CaptureFailure.Code.SOURCE,()->capture.advance(1));
        source.failRead=false;
        if (capture.advance(2)!=CampaignCapture.Result.IDLE || capture.published()!=null)
            throw new AssertionError("source failure discards partial stage");
        source.mutateAt=source.reads+2;capture.begin(3);
        if (capture.advance(3)!=CampaignCapture.Result.PARTIAL) throw new AssertionError("first chunk precedes mutation");
        if (capture.advance(4)!=CampaignCapture.Result.STALE || capture.published()!=null)
            throw new AssertionError("mutation within final chunk must fail final generation validation");
    }
}

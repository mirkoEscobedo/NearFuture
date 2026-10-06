package nf.capture;
final class CaptureChurnTest {
    private CaptureChurnTest() { }
    static void run() {
        CaptureSmoke.SyntheticSource source=new CaptureSmoke.SyntheticSource();
        CampaignCapture capture=new CampaignCapture(source,Thread.currentThread(),1);
        for (int frame=0;frame<8;frame++) {
            if (!capture.begin(frame)) throw new AssertionError("initial retries admitted");
            source.generation++;
            if (capture.advance(frame)!=CampaignCapture.Result.STALE) throw new AssertionError("dirty frontier rejected");
        }
        if (capture.begin(8)) throw new AssertionError("persistent churn must back off");
        if (!capture.begin(40)) throw new AssertionError("bounded backoff permits latest frontier");
        if (capture.advance(40)!=CampaignCapture.Result.PARTIAL || capture.advance(41)!=CampaignCapture.Result.PUBLISHED)
            throw new AssertionError("stable frontier publishes after churn");
    }
}

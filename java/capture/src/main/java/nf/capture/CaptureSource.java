package nf.capture;
import nf.capture.Values.Id;
import nf.capture.Values.Input;
import nf.capture.Values.SourceStamp;
/** Volatile live boundary. Only the owned campaign thread may invoke it. */
public interface CaptureSource {
    SourceStamp stamp();
    Input read(Id aggregate);
}

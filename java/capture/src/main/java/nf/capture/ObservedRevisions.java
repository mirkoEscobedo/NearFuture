package nf.capture;
import java.util.List;
import java.util.TreeMap;
import java.util.TreeSet;
import static nf.capture.Values.*;
/** Records only explicitly observed hooks. Native writer completeness remains unproven. */
public final class ObservedRevisions {
    private final Thread owner;
    private Context context;
    private long generation;
    private boolean active=true;
    private final TreeMap<Id,Long> revisions=new TreeMap<>();
    private final TreeSet<Id> dirty=new TreeSet<>();
    public ObservedRevisions(Thread owner,SourceStamp initial) {
        if (owner==null||initial==null) Values.fail(); this.owner=owner; reset(initial);
    }
    private void thread() {if (Thread.currentThread()!=owner) throw new CaptureFailure(CaptureFailure.Code.WRONG_THREAD);}
    private void ensureActive() {if (!active) throw new CaptureFailure(CaptureFailure.Code.INACTIVE);}
    private void reset(SourceStamp stamp) {
        thread();context=stamp.context();generation=stamp.generation();revisions.clear();dirty.clear();
        for (Revision revision:stamp.readSet()) {revisions.put(revision.aggregate(),revision.revision());dirty.add(revision.aggregate());}
    }
    public SourceStamp stamp() {
        thread();ensureActive();return new SourceStamp(context,generation,revisions.entrySet().stream().map(entry->new Revision(entry.getKey(),entry.getValue())).toList());
    }
    public void observe(Id aggregate) {
        thread();ensureActive();Long previous=revisions.get(aggregate);
        if (previous==null) Values.fail();
        if (previous==Long.MAX_VALUE||generation==Long.MAX_VALUE) {active=false;throw new CaptureFailure(CaptureFailure.Code.LIMIT);}
        revisions.put(aggregate,previous+1);generation++;dirty.add(aggregate);
    }
    public List<Id> dirty() {thread();ensureActive();return List.copyOf(dirty);}
    public void acknowledge(SourceStamp captured) {
        thread();ensureActive();if (!stamp().equals(captured)) throw new CaptureFailure(CaptureFailure.Code.DRIFT);dirty.clear();
    }
    public void replace(Context next,List<Revision> readSet) {
        thread();ensureActive();if (generation==Long.MAX_VALUE) {active=false;throw new CaptureFailure(CaptureFailure.Code.LIMIT);}
        reset(new SourceStamp(next,generation+1,readSet));
    }
}

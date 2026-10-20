package nf.contract.system;

import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import nf.contract.canonical.Canonical;
import nf.contract.canonical.Validation;

/** NF-SPT-1 data only. No native object construction or publication authority. */
public final class SystemPackageTopology {
    private SystemPackageTopology() { }
    private static final byte[] HEADER = {'N','F','-','S','P','T','-','1',0,1,0};
    public static final Limits DEFAULT = new Limits(128,64,128,65536);
    public enum Code { INVALID, LIMIT, CYCLE, PRIVACY }
    public static final class Failure extends IllegalArgumentException {
        private static final long serialVersionUID = 1L;
        private final Code code;
        public Failure(Code code) { super(code.name()); this.code = code; }
        public Code code() { return code; }
    }
    private static Failure fail(Code code) { return new Failure(code); }
    public record Limits(int entities, int dependencies, int labelBytes, int documentBytes) { }
    private static void limits(Limits v) {
        if (v == null) throw fail(Code.INVALID);
        if (v.entities()<0 || v.entities()>128 || v.dependencies()<0 || v.dependencies()>64
                || v.labelBytes()<0 || v.labelBytes()>128 || v.documentBytes()<0 || v.documentBytes()>65536) throw fail(Code.LIMIT);
    }
    public record Id(byte[] value) {
        public Id { if (value==null || value.length!=16) throw fail(Code.INVALID); value=value.clone(); }
        @Override public byte[] value() { return value.clone(); }
        @Override public boolean equals(Object other) { return other instanceof Id id && Arrays.equals(value,id.value); }
        @Override public int hashCode() { return Arrays.hashCode(value); }
    }
    public record Key(byte[] value) {
        public Key { if (value==null || value.length!=32) throw fail(Code.INVALID); value=value.clone(); }
        @Override public byte[] value() { return value.clone(); }
        @Override public boolean equals(Object other) { return other instanceof Key key && Arrays.equals(value,key.value); }
        @Override public int hashCode() { return Arrays.hashCode(value); }
    }
    public enum Kind { STAR, PLANET, STATION }
    public record Dependency(Key semanticKey, boolean required) { }
    public record Entity(Id id, Kind kind, Id parent, Integer dependency, String label) { }
    public record Topology(Id universe, Id history, Id owner, Id system, long revision, long campaignDayMillis,
            List<Dependency> dependencies, List<Entity> entities) {
        public Topology {
            if (dependencies==null || entities==null) throw fail(Code.INVALID);
            dependencies=List.copyOf(dependencies); entities=List.copyOf(entities);
        }
    }
    public record ContentStatus(List<Key> missingRequired) {
        public ContentStatus { missingRequired=List.copyOf(missingRequired); }
        public boolean abstractOnly() { return !missingRequired.isEmpty(); }
    }
    private static boolean nonzero(byte[] v) { for (byte b:v) if (b!=0) return true; return false; }
    private static int compare(Id a, Id b) { return Arrays.compareUnsigned(a.value,b.value); }
    private static int compare(Key a, Key b) { return Arrays.compareUnsigned(a.value,b.value); }
    private static int entityIndex(List<Entity> rows, Id id) {
        int low=0,high=rows.size();
        while (low<high) { int mid=(low+high)>>>1; int cmp=compare(rows.get(mid).id(),id); if(cmp<0) low=mid+1; else high=mid; }
        return low<rows.size() && rows.get(low).id().equals(id) ? low : -1;
    }
    private static int keyIndex(List<Key> rows, Key key) {
        int low=0,high=rows.size();
        while(low<high) { int mid=(low+high)>>>1; if(compare(rows.get(mid),key)<0) low=mid+1; else high=mid; }
        return low<rows.size() && rows.get(low).equals(key) ? low : -1;
    }
    private static int idIndex(List<Id> rows, Id id) {
        int low=0,high=rows.size();
        while(low<high) { int mid=(low+high)>>>1; if(compare(rows.get(mid),id)<0) low=mid+1; else high=mid; }
        return low<rows.size() && rows.get(low).equals(id) ? low : -1;
    }
    private static void keys(List<Key> rows, Limits limits) {
        if(rows==null) throw fail(Code.INVALID);
        if(rows.size()>limits.dependencies()) throw fail(Code.LIMIT);
        Key previous=null;
        for(Key key:rows) { if(key==null || !nonzero(key.value) || previous!=null && compare(previous,key)>=0) throw fail(Code.INVALID); previous=key; }
    }
    private static byte[] label(String text, Limits limits) {
        if(text.length()>limits.labelBytes()) throw fail(Code.LIMIT);
        byte[] bytes=text.getBytes(StandardCharsets.UTF_8);
        if(bytes.length>limits.labelBytes()) throw fail(Code.LIMIT);
        if(limits.labelBytes()==0) return bytes; // Earlier bounds prove the label is empty.
        try { return Canonical.validateText(text,limits.labelBytes()); }
        catch(Validation error) { throw fail(Code.INVALID); }
    }
    /** Iterative logical graph bound128 is independent of NF-CANON-1 recursive depth32. */
    public static void validate(Topology value, Limits limits) {
        limits(limits);
        if(value==null) throw fail(Code.INVALID);
        if(value.entities().isEmpty() || value.entities().size()>limits.entities() || value.dependencies().size()>limits.dependencies()) throw fail(Code.LIMIT);
        if(value.revision()==0 || value.universe()==null || value.history()==null || value.owner()==null || value.system()==null
                || !nonzero(value.universe().value) || !nonzero(value.history().value) || !nonzero(value.owner().value) || !nonzero(value.system().value)) throw fail(Code.INVALID);
        Key previousKey=null;
        for(Dependency d:value.dependencies()) {
            if(d.semanticKey()==null || !nonzero(d.semanticKey().value) || previousKey!=null && compare(previousKey,d.semanticKey())>=0) throw fail(Code.INVALID);
            previousKey=d.semanticKey();
        }
        Id previousId=null;
        for(Entity e:value.entities()) {
            if(e.id()==null || !nonzero(e.id().value) || previousId!=null && compare(previousId,e.id())>=0 || e.kind()==null) throw fail(Code.INVALID);
            previousId=e.id();
        }
        int[] parents=new int[128]; Arrays.fill(parents,-1); int stars=0;
        for(int i=0;i<value.entities().size();i++) {
            Entity e=value.entities().get(i);
            if(e.label()!=null) label(e.label(),limits);
            if(e.dependency()!=null && (e.dependency()<0 || e.dependency()>=value.dependencies().size())) throw fail(Code.INVALID);
            if(e.kind()==Kind.STAR && e.parent()==null) stars++;
            else if((e.kind()==Kind.PLANET || e.kind()==Kind.STATION) && e.parent()!=null) {
                parents[i]=entityIndex(value.entities(),e.parent()); if(parents[i]<0) throw fail(Code.INVALID);
            } else throw fail(Code.INVALID);
        }
        if(stars!=1) throw fail(Code.INVALID);
        for(int start=0;start<value.entities().size();start++) {
            boolean[] seen=new boolean[128]; int next=start,depth=0;
            while(next>=0) { if(seen[next]) throw fail(Code.CYCLE); if(++depth>128) throw fail(Code.LIMIT); seen[next]=true; next=parents[next]; }
        }
        if(measured(value)>limits.documentBytes()) throw fail(Code.LIMIT);
    }
    private static int measured(Topology value) {
        int size=96+value.dependencies().size()*33;
        for(Entity e:value.entities()) size+=20+(e.parent()==null?0:16)+(e.dependency()==null?0:2)+(e.label()==null?0:2+e.label().getBytes(StandardCharsets.UTF_8).length);
        return size;
    }
    public static Topology select(Topology source, List<Id> selected, List<Key> allowedKeys, Limits limits) {
        validate(source,limits); keys(allowedKeys,limits);
        if(selected==null) throw fail(Code.INVALID);
        if(selected.isEmpty() || selected.size()>limits.entities()) throw fail(Code.PRIVACY);
        Id previous=null;
        for(Id id:selected) { if(id==null || previous!=null && compare(previous,id)>=0) throw fail(Code.INVALID); previous=id; }
        ArrayList<Entity> entities=new ArrayList<>(selected.size()); boolean[] used=new boolean[64];
        for(Id id:selected) {
            int index=entityIndex(source.entities(),id); if(index<0) throw fail(Code.PRIVACY);
            Entity e=source.entities().get(index);
            if(e.parent()!=null && idIndex(selected,e.parent())<0) throw fail(Code.PRIVACY);
            if(e.dependency()!=null) { Key key=source.dependencies().get(e.dependency()).semanticKey(); if(keyIndex(allowedKeys,key)<0) throw fail(Code.PRIVACY); used[e.dependency()]=true; }
            entities.add(e);
        }
        ArrayList<Dependency> dependencies=new ArrayList<>(); int[] remap=new int[64]; Arrays.fill(remap,-1);
        for(int i=0;i<source.dependencies().size();i++) if(used[i]) { remap[i]=dependencies.size(); dependencies.add(source.dependencies().get(i)); }
        for(int i=0;i<entities.size();i++) { Entity e=entities.get(i); if(e.dependency()!=null) entities.set(i,new Entity(e.id(),e.kind(),e.parent(),remap[e.dependency()],e.label())); }
        Topology output=new Topology(source.universe(),source.history(),source.owner(),source.system(),source.revision(),source.campaignDayMillis(),dependencies,entities);
        validate(output,limits); return output;
    }
    public static ContentStatus diagnoseContent(Topology value, List<Key> knownKeys, Limits limits) {
        validate(value,limits); keys(knownKeys,limits); ArrayList<Key> missing=new ArrayList<>();
        for(Dependency d:value.dependencies()) if(d.required() && keyIndex(knownKeys,d.semanticKey())<0) missing.add(d.semanticKey());
        return new ContentStatus(missing);
    }
    private static void number(ByteBuffer out,long n,int bytes) { for(int i=0;i<bytes;i++) out.put((byte)(n>>>(8*i))); }
    public static byte[] encode(Topology value, Limits limits) {
        validate(value,limits); ByteBuffer out=ByteBuffer.allocate(measured(value)); out.put(HEADER);
        for(Id id:List.of(value.universe(),value.history(),value.owner(),value.system())) out.put(id.value);
        number(out,value.revision(),8); number(out,value.campaignDayMillis(),8); number(out,1,1); number(out,value.dependencies().size(),2);
        for(Dependency d:value.dependencies()) { out.put(d.semanticKey().value); number(out,d.required()?1:0,1); }
        number(out,value.entities().size(),2);
        for(Entity e:value.entities()) {
            out.put(e.id().value); number(out,e.kind().ordinal()+1,1);
            number(out,e.parent()==null?0:1,1); if(e.parent()!=null) out.put(e.parent().value);
            number(out,e.dependency()==null?0:1,1); if(e.dependency()!=null) number(out,e.dependency(),2);
            number(out,e.label()==null?0:1,1); if(e.label()!=null) { byte[] bytes=label(e.label(),limits); number(out,bytes.length,2); out.put(bytes); }
        }
        return out.array();
    }
    public static byte[] digest(Topology value, Limits limits) { return Canonical.sha256(encode(value,limits)); }
    private static final class Reader {
        private final byte[] input; private int offset;
        Reader(byte[] input) { this.input=input.clone(); }
        byte[] take(int count) { if(count<0 || count>input.length-offset) throw fail(Code.INVALID); byte[] result=Arrays.copyOfRange(input,offset,offset+count); offset+=count; return result; }
        long number(int count) { byte[] bytes=take(count); long result=0; for(int i=0;i<count;i++) result|=(bytes[i]&255L)<<(8*i); return result; }
        boolean flag() { long v=number(1); if(v>1) throw fail(Code.INVALID); return v==1; }
        int count(int maximum,int minimum) { int n=(int)number(2); if(n>maximum) throw fail(Code.LIMIT); if(n>(input.length-offset)/minimum) throw fail(Code.INVALID); return n; }
    }
    public static Topology decode(byte[] input, Limits limits) {
        limits(limits); if(input==null) throw fail(Code.INVALID); if(input.length>limits.documentBytes()) throw fail(Code.LIMIT);
        Reader reader=new Reader(input); if(!Arrays.equals(reader.take(HEADER.length),HEADER)) throw fail(Code.INVALID);
        Id universe=new Id(reader.take(16)),history=new Id(reader.take(16)),owner=new Id(reader.take(16)),system=new Id(reader.take(16));
        long revision=reader.number(8),date=reader.number(8); if(reader.number(1)!=1) throw fail(Code.INVALID);
        int count=reader.count(limits.dependencies(),33); ArrayList<Dependency> dependencies=new ArrayList<>(count);
        for(int i=0;i<count;i++) dependencies.add(new Dependency(new Key(reader.take(32)),reader.flag()));
        count=reader.count(limits.entities(),20); ArrayList<Entity> entities=new ArrayList<>(count);
        for(int i=0;i<count;i++) {
            Id id=new Id(reader.take(16)); int kind=(int)reader.number(1); if(kind<1 || kind>3) throw fail(Code.INVALID);
            Id parent=reader.flag()?new Id(reader.take(16)):null;
            Integer dependency=reader.flag()?(int)reader.number(2):null; String label=null;
            if(reader.flag()) {
                int n=(int)reader.number(2); if(n>limits.labelBytes()) throw fail(Code.LIMIT);
                byte[] bytes=reader.take(n);
                try { label=StandardCharsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString(); }
                catch(CharacterCodingException error) { throw fail(Code.INVALID); }
            }
            entities.add(new Entity(id,Kind.values()[kind-1],parent,dependency,label));
        }
        if(reader.offset!=reader.input.length) throw fail(Code.INVALID);
        Topology value=new Topology(universe,history,owner,system,revision,date,dependencies,entities); validate(value,limits); return value;
    }
}

package nf.contract.system;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import static nf.contract.system.SystemPackageTopology.*;

/** Independent literal and public failure controls. No game or private fixture. */
public final class SystemPackageTopologyTest {
    private SystemPackageTopologyTest() { }
    private static final String GOLDEN = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String SHA = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
    private static byte[] repeated(int n,int count) { byte[] v=new byte[count];Arrays.fill(v,(byte)n);return v; }
    private static Id id(int n) { return new Id(repeated(n,16)); }
    private static Key key(int n) { return new Key(repeated(n,32)); }
    private static byte[] bytes(String hex) { byte[] v=new byte[hex.length()/2];for(int i=0;i<v.length;i++)v[i]=(byte)Integer.parseInt(hex.substring(i*2,i*2+2),16);return v; }
    private static Topology fixture() {
        return new Topology(id(0x11),id(0x22),id(0x33),id(0x44),1,2,
            List.of(new Dependency(key(0x77),true)),List.of(new Entity(id(1),Kind.STAR,null,null,"Sun"),new Entity(id(2),Kind.PLANET,id(1),0,"World")));
    }
    private static Topology rows(Topology v,List<Dependency> dependencies,List<Entity> entities) { return new Topology(v.universe(),v.history(),v.owner(),v.system(),v.revision(),v.campaignDayMillis(),dependencies,entities); }
    private static void check(boolean v) { if(!v)throw new AssertionError("public topology assertion"); }
    private static void refuses(Code code,Runnable action) { try {action.run();}catch(Failure error){check(error.code()==code);return;}throw new AssertionError("expected "+code); }
    private static void first() {
        Topology v=fixture();Topology cycle=rows(v,v.dependencies(),List.of(v.entities().get(0),new Entity(id(2),Kind.PLANET,id(3),0,"World"),new Entity(id(3),Kind.STATION,id(2),null,null)));
        refuses(Code.CYCLE,()->validate(cycle,DEFAULT));refuses(Code.CYCLE,()->encode(cycle,DEFAULT));
        refuses(Code.CYCLE,()->decode(bytes("4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770103000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020103030303030303030303030303030303010000010500576f726c64030303030303030303030303030303030301020202020202020202020202020202020000"),DEFAULT));
        check(bytes(GOLDEN).length==199);check(decode(bytes(GOLDEN),DEFAULT).equals(v));check(Arrays.equals(encode(v,DEFAULT),bytes(GOLDEN)));check(Arrays.equals(digest(v,DEFAULT),bytes(SHA)));
    }
    private static void malformed() {
        int[] offsets={9,91,92,127,126,192},replacements={2,2,65,129,2,129};Code[] errors={Code.INVALID,Code.INVALID,Code.LIMIT,Code.LIMIT,Code.INVALID,Code.LIMIT};
        for(int i=0;i<offsets.length;i++){byte[] v=bytes(GOLDEN);v[offsets[i]]=(byte)replacements[i];Code code=errors[i];refuses(code,()->decode(v,DEFAULT));}
        byte[] zero=bytes(GOLDEN);Arrays.fill(zero,75,83,(byte)0);refuses(Code.INVALID,()->decode(zero,DEFAULT));
        byte[] trailing=Arrays.copyOf(bytes(GOLDEN),200);refuses(Code.INVALID,()->decode(trailing,DEFAULT));
        byte[] shortInput=Arrays.copyOf(bytes(GOLDEN),198);refuses(Code.INVALID,()->decode(shortInput,DEFAULT));
        refuses(Code.LIMIT,()->decode(new byte[65537],DEFAULT));
    }
    private static void referencesAndText() {
        Topology v=fixture();Entity star=v.entities().get(0);
        for(Entity invalid:List.of(new Entity(id(2),Kind.PLANET,id(9),0,"World"),new Entity(id(1),Kind.PLANET,id(1),0,"World"),new Entity(id(2),Kind.PLANET,null,0,"World"))) {
            Topology bad=rows(v,v.dependencies(),List.of(star,invalid));refuses(Code.INVALID,()->validate(bad,DEFAULT));
        }
        Topology duplicate=rows(v,List.of(v.dependencies().get(0),v.dependencies().get(0)),v.entities());refuses(Code.INVALID,()->validate(duplicate,DEFAULT));
        Topology nonNfc=rows(v,v.dependencies(),List.of(star,new Entity(id(2),Kind.PLANET,id(1),0,"e\u0301")));refuses(Code.INVALID,()->validate(nonNfc,DEFAULT));
        Topology maxLabel=rows(v,v.dependencies(),List.of(star,new Entity(id(2),Kind.PLANET,id(1),0,"x".repeat(128))));validate(maxLabel,DEFAULT);check(decode(encode(maxLabel,DEFAULT),DEFAULT).equals(maxLabel));
        Topology longLabel=rows(v,v.dependencies(),List.of(star,new Entity(id(2),Kind.PLANET,id(1),0,"x".repeat(129))));refuses(Code.LIMIT,()->validate(longLabel,DEFAULT));
    }
    private static void bounds() {
        Topology v=fixture();ArrayList<Entity> entities=new ArrayList<>();ArrayList<Dependency> dependencies=new ArrayList<>();
        for(int n=1;n<=128;n++)entities.add(new Entity(id(n),n==1?Kind.STAR:Kind.PLANET,n==1?null:id(n-1),null,null));
        for(int n=1;n<=64;n++)dependencies.add(new Dependency(key(n),true));Topology maximum=rows(v,dependencies,entities);
        validate(maximum,DEFAULT);check(decode(encode(maximum,DEFAULT),DEFAULT).equals(maximum));refuses(Code.LIMIT,()->validate(maximum,new Limits(127,64,128,65536)));
        entities.add(new Entity(id(129),Kind.PLANET,id(128),null,null));Topology tooMany=rows(v,dependencies,entities);refuses(Code.LIMIT,()->validate(tooMany,DEFAULT));
        dependencies.add(new Dependency(key(65),true));Topology tooManyKeys=rows(v,dependencies,v.entities());refuses(Code.LIMIT,()->validate(tooManyKeys,DEFAULT));
        Limits zeroLabel=new Limits(128,64,0,65536);
        Topology emptyLabels=rows(v,v.dependencies(),List.of(new Entity(id(1),Kind.STAR,null,null,""),new Entity(id(2),Kind.PLANET,id(1),0,"")));
        validate(emptyLabels,zeroLabel);check(decode(encode(emptyLabels,zeroLabel),zeroLabel).equals(emptyLabels));
        Topology nonempty=rows(v,v.dependencies(),List.of(new Entity(id(1),Kind.STAR,null,null,"x"),emptyLabels.entities().get(1)));refuses(Code.LIMIT,()->encode(nonempty,zeroLabel));
        refuses(Code.LIMIT,()->encode(v,new Limits(128,64,129,65536)));refuses(Code.LIMIT,()->encode(v,new Limits(128,64,128,198)));
    }
    private static void privacy() {
        Topology v=fixture();refuses(Code.PRIVACY,()->select(v,List.of(id(2)),List.of(key(0x77)),DEFAULT));refuses(Code.PRIVACY,()->select(v,List.of(id(1),id(2)),List.of(),DEFAULT));
        Topology output=select(v,List.of(id(1)),List.of(),DEFAULT);check(output.entities().equals(List.of(v.entities().get(0))));check(output.dependencies().isEmpty());
        check(!new String(encode(output,DEFAULT),java.nio.charset.StandardCharsets.ISO_8859_1).contains("World"));
        refuses(Code.PRIVACY,()->select(v,List.of(id(1),id(9)),List.of(),DEFAULT));refuses(Code.INVALID,()->select(v,List.of(id(1),id(1)),List.of(),DEFAULT));
        Topology remap=rows(v,List.of(new Dependency(key(0x55),false),new Dependency(key(0x77),true)),List.of(v.entities().get(0),new Entity(id(2),Kind.PLANET,id(1),1,"World")));
        check(select(remap,List.of(id(1),id(2)),List.of(key(0x77)),DEFAULT).equals(v));
    }
    private static void contentAndIdentity() {
        Topology v=fixture();ContentStatus unknown=diagnoseContent(v,List.of(),DEFAULT);check(unknown.abstractOnly());check(unknown.missingRequired().equals(List.of(key(0x77))));
        check(!diagnoseContent(v,List.of(key(0x77)),DEFAULT).abstractOnly());Topology optional=rows(v,List.of(new Dependency(key(0x77),false)),v.entities());check(!diagnoseContent(optional,List.of(),DEFAULT).abstractOnly());
        Topology changed=new Topology(v.universe(),v.history(),v.owner(),v.system(),2,v.campaignDayMillis(),v.dependencies(),v.entities());check(!Arrays.equals(digest(v,DEFAULT),digest(changed,DEFAULT)));
        Topology empty=rows(v,v.dependencies(),List.of(new Entity(id(1),Kind.STAR,null,null,""),v.entities().get(1))),absent=rows(v,v.dependencies(),List.of(new Entity(id(1),Kind.STAR,null,null,null),v.entities().get(1)));check(!Arrays.equals(encode(empty,DEFAULT),encode(absent,DEFAULT)));
        Id original=v.entities().get(0).id();byte[] borrowed=original.value();borrowed[0]=9;check(original.equals(id(1)));
    }
    public static void main(String[] args) { if(args.length!=0)throw new AssertionError("no arguments");first();malformed();referencesAndText();bounds();privacy();contentAndIdentity();System.out.println("NF-SPT-1 six public groups PASS; data only"); }
}

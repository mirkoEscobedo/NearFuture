package nf.nex.reference;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
final class DifferentialWarReader {
    private DifferentialWarReader() { }
    static WarInput input(String[] f) {
        if(f.length!=13) throw new IllegalArgumentException("Invalid public corpus shape");
        var traits=new HashSet<String>();int mask=Integer.parseInt(f[6]);if(mask<0||mask>7) throw new IllegalArgumentException("Invalid traits");
        if((mask&1)!=0) traits.add("pacifist");if((mask&2)!=0) traits.add("weak-willed");if((mask&4)!=0) traits.add("stalwart");
        var existing=new ArrayList<WarInput.ExistingConcern>();
        switch(f[5]) {case "none" -> { }case "active" -> existing.add(new WarInput.ExistingConcern(WarInput.ConcernClass.WAR_WEARINESS,false));case "ended" -> existing.add(new WarInput.ExistingConcern(WarInput.ConcernClass.WAR_WEARINESS,true));case "other" -> existing.add(new WarInput.ExistingConcern(WarInput.ConcernClass.OTHER,false));default -> throw new IllegalArgumentException("Invalid class");}
        var priority=new PriorityInputs(0.5f,traits,f[7].equals("-")?null:value(f[7]),0.25f,1.3f,0.7f,List.of("diplomacy","canMakePeace","trait_pacifist","trait_weak-willed","!trait_stalwart"),List.of(new PriorityEntry("value",PriorityEntry.Kind.FLAT,0x3f800000)));
        return new WarInput(value(f[1]),value(f[2]),bool(f[3]),bool(f[4]),existing,priority);
    }
    static boolean bool(String value) {if(!value.equals("true")&&!value.equals("false")) throw new IllegalArgumentException("Invalid boolean");return value.equals("true");}
    static float value(String hex) {if(!hex.matches("[0-9a-f]{8}")) throw new IllegalArgumentException("Invalid float");return Float.intBitsToFloat(Integer.parseUnsignedInt(hex,16));}
}
package nf.nex.reference;
import java.util.ArrayList;
final class DifferentialPeaceReader {
    private DifferentialPeaceReader() { }
    static PeaceInput input(String[] f) {
        if(f.length!=8) throw new IllegalArgumentException("Invalid public corpus shape");String gate=f[1];
        if(!java.util.Set.of("none","noEnemies","pirate","warInterval","ownWeariness","enemyWeariness","recentWar","canCeasefire","commissioned","offensive").contains(gate)) throw new IllegalArgumentException("Invalid gate");
        var draws=new ArrayList<SuppliedDraw>();draws.add(new SuppliedDraw("peace-chance",bits(f[3])));if(!f[4].equals("-")) draws.add(new SuppliedDraw("peace-treaty",bits(f[4])));
        var gates=new PeaceGates(!gate.equals("noEnemies"),gate.equals("pirate"),false,gate.equals("warInterval")?Float.intBitsToFloat(0x41efffff):30f,30f,gate.equals("recentWar"),!gate.equals("canCeasefire"),gate.equals("commissioned"),gate.equals("offensive"),true);
        var rules=new PeaceRules(5000f,20000f,75f,0,40f,0.3f,3000f,6000f);float below=Float.intBitsToFloat(0x459c3fff);
        return new PeaceInput("hegemony","tritachyon",false,DifferentialWarReader.bool(f[2]),gate.equals("ownWeariness")?below:5000f,gate.equals("enemyWeariness")?below:5000f,0,0,gates,rules,draws);
    }
    private static long bits(String hex) {if(!hex.matches("[0-9a-f]{16}")) throw new IllegalArgumentException("Invalid double");return Long.parseUnsignedLong(hex,16);}
}
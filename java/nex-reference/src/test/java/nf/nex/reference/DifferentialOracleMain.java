package nf.nex.reference;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.charset.StandardCharsets;
import java.util.Locale;
import java.util.stream.Collectors;
/** Independent Java oracle: public raw input columns only; golden expected columns are never read. */
public final class DifferentialOracleMain {
    private DifferentialOracleMain() { }
    public static void main(String[] args) {
        try {run(args);}catch(Exception rejected) {System.err.println("REFERENCE_UNAVAILABLE");System.exit(1);}
    }
    private static void run(String[] args) throws Exception {
        if(args.length!=2||!java.util.Set.of("war","peace","action","traversal","traversal_targeted","first_proposal").contains(args[0])) throw new IllegalArgumentException("Invalid corpus mode");
        byte[] bytes;try(var input=Files.newInputStream(Path.of(args[1]))) {bytes=input.readNBytes(65537);}if(bytes.length>65536) throw new IllegalArgumentException("Corpus limit");
        String text=new String(bytes,StandardCharsets.UTF_8);if(!java.util.Arrays.equals(bytes,text.getBytes(StandardCharsets.UTF_8))) throw new IllegalArgumentException("Invalid UTF8");
        StringBuilder output=new StringBuilder();int rows=0;
        for(String row:text.split("\\R")) {
            if(row.isBlank()||row.startsWith("#")) continue;if(row.length()>1024||++rows>256) throw new IllegalArgumentException("Corpus limit");
            String[] f=row.split("\\|",-1);if(!f[0].matches("[a-z_]{1,64}")) throw new IllegalArgumentException("Invalid case identifier");
            if(args[0].equals("war")) {
                WarInput input=DifferentialWarReader.input(f);WarResult result=WarWearinessReference.generate(input);
                output.append("war|").append(f[0]).append('|').append(result.generated()).append('|').append(result.ended()).append('|').append(result.abortCurrentAction()).append('|').append(WarWearinessReference.isValid(input)).append('|').append(priority(result.existingPriority())).append('|').append(priority(result.writes())).append('\n');
            } else if(args[0].equals("traversal")) {
                output.append("traversal|").append(f[0]).append('|').append(DifferentialTraversalReader.output(f)).append('\n');
            } else if(args[0].equals("traversal_targeted")) {
                output.append("traversal_targeted|").append(f[0]).append('|').append(DifferentialTraversalReader.outputSelection(f)).append('\n');
            } else if(args[0].equals("first_proposal")) {
                output.append("first_proposal|").append(f[0]).append('|').append(DifferentialFirstProposalReader.output(f)).append('\n');
            } else if(args[0].equals("action")) {
                if(f.length!=6) throw new IllegalArgumentException("Invalid public corpus shape");
                boolean eligible=PeaceReference.canUseAction(DifferentialWarReader.bool(f[1]),DifferentialWarReader.bool(f[2]),DifferentialWarReader.bool(f[3]),DifferentialWarReader.bool(f[4]),DifferentialWarReader.bool(f[5]));
                output.append("action|").append(f[0]).append('|').append(eligible).append('\n');
            } else {
                PeaceResult result=PeaceReference.evaluateSelectedEnemy(DifferentialPeaceReader.input(f));
                String effects=result.effects().stream().map(effect->effect instanceof PeaceResult.DiplomacyEventCall e?"event:"+e.faction()+":"+e.enemy()+":"+e.eventId():effect instanceof PeaceResult.WearinessCall w?"weariness:"+w.faction()+":"+String.format(Locale.ROOT,"%08x",w.amountFloatBits()):"unsupported").collect(Collectors.joining(","));
                output.append("peace|").append(f[0]).append('|').append(result.decision()).append('|').append(result.consumedDraws()).append('|').append(effects.isEmpty()?"-":effects).append('\n');
            }
            if(output.length()>65536) throw new IllegalArgumentException("Output limit");
        }if(rows==0) throw new IllegalArgumentException("Empty corpus");System.out.print(output);
    }
    private static String priority(java.util.List<PriorityEntry> entries) {String result=entries.stream().map(e->e.id()+":"+e.kind()+":"+String.format(Locale.ROOT,"%08x",e.floatBits())).collect(Collectors.joining(","));return result.isEmpty()?"-":result;}
}
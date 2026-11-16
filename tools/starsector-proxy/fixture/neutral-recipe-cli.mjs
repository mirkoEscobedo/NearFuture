// @ts-check
import {readNeutralRecipe,prepareNeutralRecipe} from './neutral-recipe.mjs';

const [mode,path,hash,...extra]=process.argv.slice(2);
if(extra.length||!['inspect','prepare'].includes(mode??'')||!path||!hash){
    console.error('Usage: node neutral-recipe-cli.mjs <inspect|prepare> <private-recipe.json> <sha256>');process.exitCode=2;
} else {
    try {
        if(mode==='prepare') console.log(JSON.stringify(prepareNeutralRecipe(path,hash)));
        else {const m=readNeutralRecipe(path,hash);console.log(JSON.stringify({profile:m.profile,protocol:m.protocol,mods:m.mods.map((/** @type {any} */ x)=>({id:x.id,version:x.version})),state:'RECIPE_INSPECTED_ONLY',launchAdmitted:false,nativeAcceptance:'UNPROVED'}));}
    } catch {console.error('Neutral recipe refused; preserve private inputs and inspect the owning preparation error.');process.exitCode=1;}
}

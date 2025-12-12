const { execSync } = require("child_process");
const path = require("path");

const sizes = [
    1024,           // 1KB
    10 * 1024,      // 10KB
    100 * 1024,     // 100KB
    1024 * 1024,    // 1MB
    5 * 1024 * 1024, // 5MB (10MB might be too slow for quick bench)
    10 * 1024 * 1024 // 10MB
];

const modes = ['js', 'native'];

console.log("| Size | Mode | Encode (ops/s) | Decode (ops/s) |");
console.log("|---|---|---|---|");

for (const size of sizes) {
    const results = {};
    let actualSize = 0;

    for (const mode of modes) {
        try {
            const cmd = `node bench/run_case.js ${mode} ${size}`;
            const output = execSync(cmd, { cwd: path.join(__dirname, "..") }).toString();
            
            const lines = output.trim().split('\n');
            results[mode] = {};
            
            lines.forEach(line => {
                try {
                    const data = JSON.parse(line);
                    results[mode][data.type] = data;
                    if (data.size) actualSize = data.size;
                } catch (e) {}
            });
        } catch (e) {
            // console.error(`Error running ${mode} ${size}:`, e.message);
        }
    }
    
    // Format output
    const jsEnc = results['js']?.encode?.ops || 0;
    const jsDec = results['js']?.decode?.ops || 0;
    const natEnc = results['native']?.encode?.ops || 0;
    const natDec = results['native']?.decode?.ops || 0;
    
    const sizeStr = actualSize < 1024 * 1024 
        ? (actualSize / 1024).toFixed(2) + " KB"
        : (actualSize / 1024 / 1024).toFixed(2) + " MB";
    
    const encDiff = jsEnc ? ((natEnc - jsEnc) / jsEnc * 100).toFixed(1) + "%" : "N/A";
    const decDiff = jsDec ? ((natDec - jsDec) / jsDec * 100).toFixed(1) + "%" : "N/A";

    // Add sign to diff
    const encSign = encDiff.startsWith("-") ? "" : "+";
    const decSign = decDiff.startsWith("-") ? "" : "+";

    console.log(`| ${sizeStr} | JS | ${jsEnc.toFixed(0)} | ${jsDec.toFixed(0)} |`);
    console.log(`| | Native | ${natEnc.toFixed(0)} (${encSign}${encDiff}) | ${natDec.toFixed(0)} (${decSign}${decDiff}) |`);
}

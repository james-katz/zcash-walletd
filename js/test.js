const native = require("./dist/");

try {
    const rep = native.init();
    console.log(rep);

    const acc = native.getAccounts();
    console.log(acc);

    // const txns = native.getTransfers(0, [0]);
    // console.log(txns);

    setInterval(() => {
        const hStr = native.getHeight();
        const h = JSON.parse(hStr);
        console.log("curr chain height: ", h);
        const whStr = native.getWalletHeight();
        const wh = JSON.parse(whStr);
        console.log("prev wallet height: ", h);
    
        if(h.height > wh.height) {
            const scan = native.requestScan();
            console.log(scan);
        } 
        else {
            console.log("Wallet is already up to date");
        } 
    }, 5000);
    

    // native.requestScanAsync().then((r) => {
    //     console.log("I should run last");     
    //     console.log(r);
    // }).catch((e) => {console.log(e)});

    // console.log("I should run first");
    // setInterval(() => {
    //     let h = native.syncInfo();
    //     console.log("curr wallet height: ", h);
    // }, 5000);

    // const addrs = native.getAddresses();
    // console.log(addrs);

} catch(e) {
    console.log(e);
}

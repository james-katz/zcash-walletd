const native = require("./dist/");

try {
    const rep = native.init();
    console.log(rep);

    const acc = native.getAccounts();
    console.log(acc);

    const txns = native.getTransfers(0, [0]);
    console.log(txns);

    let h = native.getWalletHeight();
    console.log("prev wallet height: ", h);
    native.requestScan();
    // native.requestScanAsync().then((r) => {
    //     console.log("I should run last");     
    //     console.log(r);
    // }).catch((e) => {console.log(e)});

    // console.log("I should run first");
    // setInterval(() => {
    //     let h = native.syncInfo();
    //     console.log("curr wallet height: ", h);
    // }, 5000);

    const addrs = native.getAddresses();
    console.log(addrs);

} catch(e) {
    console.log(e);
}

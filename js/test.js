const native = require("./dist/");

try {
    const rep = native.init();
    console.log(rep);

    const acc = native.getAccounts();
    console.log(acc);

    const txns = native.getTransfers(0, [1]);
    console.log(txns);

    let h = native.getWalletHeight();
    console.log("prev wallet height: ", h);

    native.requestScanAsync().then((r) => {
        console.log("I should run last");     
        console.log(r);
    }).catch((e) => {console.log(e)});

    console.log("I should run first");
    // setInterval(() => {
    //     let h = native.getWalletHeight();
    //     console.log("curr wallet height: ", h);
    // }, 1000);

} catch(e) {
    console.log(e);
}

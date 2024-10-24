use mf1::msg::*;
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg
    }

    // generate a valid json
    let msg = ExecuteMsg::ExecuteTx {
        fcross_tx: FcrossTx {
            tx_id: 1,
            operation: Operation::CreditBalance { amount: 100 },
        },
    };
    let json = serde_json::to_string(&msg).unwrap();
    println!("{}", json);
    let msg2 = QueryMsg::MyLogs {  };
    let json2 = serde_json::to_string(&msg2).unwrap();
    println!("{}", json2);
    let msg3 = QueryMsg::Multifuture { tx_id: 1 };
    let json3 = serde_json::to_string(&msg3).unwrap();
    println!("{}", json3);
}

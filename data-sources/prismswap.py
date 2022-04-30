#!/usr/bin/env python3

import requests
import sys

def adjust_rounding(data):
    if data < 1:
        return round(data, 8)
    elif data < 10:
        return round(data, 6)
    else:
        return round(data, 4)

PRISMSWAP_FACTORY_ADDRESS = "terra1sfw7vvwhsczkeje22ramy76pj5cm9gtvvnzn94"

ASSET_MAP = {
    "PRISM": "terra1dh9478k2qvqhqeajhn75a2a7dsnf74y5ukregw",
    "CLUNA": "terra13zaagrrrxj47qjwczsczujlvnnntde7fdt0mau",
    "PLUNA": "terra1tlgelulz9pdkhls6uglfn5lmxarx7f2gxtdzh2",
    "YLUNA": "terra17wkadg0tah554r35x6wvff0y5s7ve8npcjfuhz",
}

PAIR_MAP = {
    "PRISM": "terra19d2alknajcngdezrdhq40h6362k92kz23sz62u",
    "CLUNA": "terra1yxgq5y6mw30xy9mmvz9mllneddy9jaxndrphvk",
    "PLUNA": "terra1persuahr6f8fm6nyup0xjc7aveaur89nwgs5vs",
    "YLUNA": "terra1kqc65n5060rtvcgcktsxycdt2a4r67q2zlvhce",
}


def get_token_decimals(contract_addr):
    token_info = requests.get(
        f"https://lcd.terra.dev/wasm/contracts/{contract_addr}/store?query_msg={{%22token_info%22: {{}}}}"
    ).json()
    decimals = token_info["result"]["decimals"]
    return int(decimals)


def get_asset_price(symbol):
    req_url = (
        'https://lcd.terra.dev/wasm/contracts/%s/store?query_msg={"reverse_simulation":{"ask_asset":{"info":{"cw20":"%s"},"amount":"1000000"}}}'
        % (PAIR_MAP[symbol], ASSET_MAP[symbol])
    )
    res = requests.get(req_url).json()
    offer_amount = res["result"]["offer_amount"]
    return int(offer_amount) / 1e6


def get_price(symbol):
    return (
        get_asset_price(symbol)
        if symbol == "PRISM"
        else get_asset_price(symbol) * get_asset_price("PRISM")
    )


def main(symbols):
    return ",".join([str(adjust_rounding(get_price(symbol))) for symbol in symbols])


if __name__ == "__main__":
    try:
        print(main(sys.argv[1:]))
    except Exception as e:
        print(e, file=sys.stderr)
        sys.exit(1)
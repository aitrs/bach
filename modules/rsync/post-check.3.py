import os
import datetime

def get_day():
    trad = {
            0: 'Monday',
            1: 'Tuesday',
            2: 'Wednesday',
            3: 'Thurday',
            4: 'Friday',
            5: 'Saturday',
            6: 'Sunday'
    }

    return trad.get(datetime.datetime.today().weekday())

if __name__ == '__main__':
    raw = os.system('ssh root@bach.dest.1 diff -r /root/compare /root/dummy-dat-weekday/{}'.format(get_day()))
    stat = os.waitstatus_to_exitcode(raw)
    if stat < 0:
        stat = stat *-1
    os.system('ssh root@bach.dest.1 rm -r /root/dummy-dat-weekday/{}/*'.format(get_day()))
    os._exit(0)

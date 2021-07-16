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

def is_mounted():
    handle = os.popen('ssh root@bach.dest.1 df')
    contents = handle.read()
    handle.close()
    pos = contents.find('/root/test2')
    return pos != -1

if __name__ == '__main__':
    if is_mounted():
        os._exit(1)
    raw = os.system('ssh root@bach.dest.1 mount -o loop,offset=1048576 /root/test.img /root/test2')
    stat = os.waitstatus_to_exitcode(raw)
    if stat != 0:
        os._exit(1)

    raw = os.system('ssh root@bach.dest.1 diff -r /root/test2/{} /root/compare'.format(get_day()))
    stat = os.waitstatus_to_exitcode(raw)
    if stat < 0:
        stat = stat * -1
    
    os.system('ssh root@bach.dest.1 rm -r /root/test2/*')
    for i in range(6):
        os.system('ssh root@bach.dest.1 mkdir /root/test2/{}'.format(get_day()))

    os.system('ssh root@bach.dest.1 umount /root/test2')

    os._exit(stat)

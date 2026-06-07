from pathlib import Path
import tomllib,psutil,pymem
import os,sys,time
import json

def custom_excepthook(exc_type, exc_value, traceback_obj):
    '''
    # 记录异常到日志文件
    logging.error("程序崩溃", exc_info=(exc_type, exc_value, traceback_obj))
    '''
    
    # 控制台输出
    sys.__excepthook__(exc_type, exc_value, traceback_obj)
    print("\n程序遇到异常，将在 5 秒后退出...")
    time.sleep(5)
    sys.exit(1)

sys.excepthook = custom_excepthook

class IDManager:
    ID_dict = {}
    ID_en_dict = {}
    Name_dict = {}
    Name_en_dict = {}
    
    def __init__(self):
        with open(r"Data\HanziKanjiDict.txt","r",encoding="UTF-8") as f:
            Hanzi_list = []
            Kanji_list = []
            for text in f.readlines():
                text_sp = text.split()
                Hanzi_list.append(text_sp[0])
                Kanji_list.append(text_sp[1])
        Hanzi_Kanji_dict = dict(zip(Hanzi_list,Kanji_list))
        self.Hanzi_Kanji = str.maketrans(Hanzi_Kanji_dict)
    
    def CheckID(self,_id):
        if _id in IDManager.ID_dict.keys():
            return True
        else:
            return False
    
    def SearchName(self,_Name:str):
        search_ans = []
        funcs = [self.__Search_Str,self.__Search_Hanzi2Kanji,self.__Search_AnotherName]
        for func in funcs:
            ans = func(_Name)
            if ans:
                search_ans += ans
                break
        ans_en = self.__Search_Str(_Name, True)
        for en_name in ans_en:
            if not en_name in search_ans:
                search_ans.append(en_name)
        return search_ans
    
    def __Search_Str(self,_Name:str, use_en:bool=False):
        return self.__SearchName(_Name, use_en)

    def __Search_Hanzi2Kanji(self,_Name:str):
        new_Name = _Name.translate(self.Hanzi_Kanji)
        return self.__SearchName(new_Name)
    
    def __Search_AnotherName(self,_Name:str):
        ans = []
        with open(r"Data\AnotherSongName.json","r",encoding="UTF-8") as f:
            AnotherNameDict = json.load(f)
        for SongName in AnotherNameDict.keys():
            if _Name.lower() in SongName.lower():
                ans += self.__SearchName(AnotherNameDict[SongName])
        return ans
    
    def __SearchName(self,_Name, use_en:bool=False):
        ans = []
        name_dict = IDManager.Name_en_dict if use_en else IDManager.Name_dict
        for SongName in name_dict.keys():
            if _Name.lower() in SongName.lower():
                ans.append(f"{SongName}：{name_dict[SongName]}\n")
        return ans
    
            
    def __GetDIVAFolder(self):
        try:
            pid = pymem.Pymem('DivaMegaMix.exe').process_id
            process = psutil.Process(pid)
            Mega_Folder = Path(process.exe()).parent
        except pymem.exception.ProcessNotFound:
            raise OSError("游戏进程不存在")
        return Mega_Folder
    
    def Read_M39ID(self):
        self.ReadPVDB(Path(r"Data\pv_db.txt"))
        DLC_CPK = self.__GetDIVAFolder().joinpath("diva_dlc00_region.cpk")
        if DLC_CPK.exists():
            self.ReadPVDB(Path(r"Data\mdata_pv_db.txt"))
    
    def GetModList(self):
        #利用游戏读取DML文件路径从而定位MOD文件夹位置
        #如果没有配置文件则默认认为没有安装DML
        #DML有 priority 定义MOD读取顺序
        #如果没有则是按照文件名顺序读取
        #暂时只处理mod_pv_db
        _path = self.__GetDIVAFolder()
        toml_path = _path.joinpath("config.toml")
        if not toml_path.exists():
            print("can't find dml config file")
            return
        with open(toml_path,"rb") as f:
            dmlconfig = tomllib.load(f)
        if not "mods" in dmlconfig:
            raise KeyError("DML config file don't have mods folder config")
        mods_path = _path.joinpath(dmlconfig["mods"])
        if "priority" in dmlconfig:
            mods_file_list = [mods_path.joinpath(item) for item in dmlconfig["priority"]]
        else:
            mods_file_list = [item for item in mods_path.iterdir() if item.is_dir()]
        mods_list = [item.joinpath(r"rom\mod_pv_db.txt") for item in mods_file_list if item.joinpath(r"rom\mod_pv_db.txt").exists()]
        for Mod in mods_list:
            self.ReadPVDB(Mod)

    def ReadPVDB(self,_file):
        with open(_file,"r",encoding="UTF-8") as f:
            text_list = f.readlines()
        New_ID_dict = self.__GetInfo(text_list)
        self.__Updata(New_ID_dict)
        New_ID_dict = self.__GetInfo_en(text_list)
        self.__Updata_en(New_ID_dict)
    
    def __Updata(self,New_ID_dict):
        IDManager.ID_dict = New_ID_dict | IDManager.ID_dict
        New_Name_dict = {}
        for key, value in IDManager.ID_dict.items():
            New_Name_dict.setdefault(value, []).append(key)
        IDManager.Name_dict = New_Name_dict

    def __Updata_en(self,New_ID_dict):
        IDManager.ID_en_dict = New_ID_dict | IDManager.ID_en_dict
        New_Name_dict = {}
        for key, value in IDManager.ID_en_dict.items():
            New_Name_dict.setdefault(value, []).append(key)
        IDManager.Name_en_dict = New_Name_dict
    
    def __GetInfo(self,text_list):
        id_list = []
        name_list = []
        for info in text_list:
            info_id = info.split(".song_name=")[0][3:]
            if info_id.isdigit() and id_list.count(int(info_id)) == 0 :
                id_list.append(int(info_id))
                name_list.append(info.split(".song_name=")[1].replace("\n",""))
        return dict(zip(id_list,name_list))
    
    def __GetInfo_en(self,text_list):
        id_list = []
        name_list = []
        for info in text_list:
            info_id = info.split(".song_name_en=")[0][3:]
            if info_id.isdigit() and id_list.count(int(info_id)) == 0 :
                id_list.append(int(info_id))
                name_list.append(info.split(".song_name_en=")[1].replace("\n",""))
        return dict(zip(id_list,name_list))

class SongSelect:
    
    def __init__(self):
        self.pm = pymem.Pymem('DivaMegaMix.exe')
        self.LastSelectPVIDMem = self.pm.base_address + int("0x12B6350" , 16)
        self.LastSelectSortMem = self.pm.base_address + int("0x12B6354" , 16)
        self.LastSelectDiffMem = self.pm.base_address + int("0x12B635C" , 16)
        self.EdenOffsetMem     = int("0x105F460" , 16)
        self.ChangeSongSelect  = self.pm.base_address + int("0xCC61098" , 16)
        self.StartChange       = self.pm.base_address + int("0xCC610A0" , 16)
        self.__EdenCheck()
        
    def __EdenCheck(self):
        if self.pm.read_int(self.LastSelectPVIDMem) == 0:
            self.LastSelectPVIDMem += self.EdenOffsetMem
            self.LastSelectSortMem += self.EdenOffsetMem
            self.LastSelectDiffMem += self.EdenOffsetMem
    
    def ChangeSong(self,_ID):
        import time
        time.sleep(5)
        if IDManager().CheckID(int(_ID)):
            if self.pm.read_int(self.ChangeSongSelect) == 6:
                self.pm.write_int(self.ChangeSongSelect, 6)
                self.pm.write_int(self.StartChange, 2)
                time.sleep(0.1)
                self.pm.write_int(self.LastSelectPVIDMem, int(_ID))
                #跳转难度
                #锁定到难度分类
                self.pm.write_int(self.LastSelectSortMem, 1)
                #锁定到ALL分类
                self.pm.write_int(self.LastSelectDiffMem, 19)
                self.pm.write_int(self.ChangeSongSelect, 5)
                self.pm.write_int(self.StartChange, 2)
            else:
                self.pm.write_int(self.LastSelectPVIDMem, int(_ID))
                self.pm.write_int(self.LastSelectSortMem, 1)
                self.pm.write_int(self.LastSelectDiffMem, 19)
            return "success!"
        else:
            return "this pvid does not exist"


SongIDManager = IDManager()
SelectManager = SongSelect()
SongIDManager.Read_M39ID()
SongIDManager.GetModList()
search_ans = ""

while True:
    os.system("cls")
    print("How to use\ninput pvid then press enter key, it's will auto jump to that song\nyou can use command to search song: -S <song name> \nexample：\n-S Love is War\n")
    print(search_ans)
    command = input("input：")
    if command.lower().find("-s") !=-1:
        SongIDManager.GetModList()
        search_ans = f"Find song：\n{''.join(SongIDManager.SearchName(command.split(maxsplit=1)[1]))}"
    if command.isdigit():
        search_ans = SelectManager.ChangeSong(command)
